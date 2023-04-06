use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashMap;
use std::fmt::format;
use std::ptr::null;
use std::sync::atomic::AtomicBool;
use log::{debug, error, info};
use r2d2_postgres::PostgresConnectionManager;
use r2d2::{Pool, PooledConnection};
use postgres::{Error, NoTls};
use crate::vars::{Var, str_to_int64};

/// Enum for device, this represents a full hopper or
/// a empty hopper
pub enum DeviceStatus{
    Empty,
    Full
}

pub enum CommunicationStatus{
    Waiting,
    Ok,
    Intermitente,
    Lost
}

pub struct HydrophoneAnalysis{
    pub id: i32,
    pub device_id: i32
}

impl HydrophoneAnalysis{
    pub fn create_lines(&self, sound_a: &str, sound_b: &str, now: NaiveDateTime, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>){
        let sound_a = sound_a.parse::<f64>().unwrap();
        let sound_b = sound_b.parse::<f64>().unwrap();
        let result = conn.execute("INSERT INTO hydrophone_analysis_line(sound_a, sound_b, analysis_id, create_date, hopper_status) VALUES ($1, $2, $3, $4, $5)", &[&sound_a, &sound_b, &self.id, &now, &"full"]);
            match result {
                Ok(_r) => debug!("Creado line hydro"),
                Err(r) => info!("Error al crear lineas {}", r)
            }
    }
}

pub struct DeviceAlarmHistory{
    pub id: i32,
    //pub alarm_counter: i32
}

/// Struct of a device
pub struct Device {
    pub id: i32,
    pub network_id: Option<i32>,
    pub name: String,
    pub address: String,
    pub protocol: String,
    pub pond_id: Option<i32>,
    pub pond_name: Option<String>,
    pub farm_id: Option<i32>,
    pub status: Option<String>,
    pub mode: Option<String>,
    pub empty_alarm: Option<bool>,
}

type Shrimp = (Option<String>, Option<i32>);

impl Device {
    /// Create a new device object
    pub fn new(id: i32, network_id: Option<i32>, name: String, address: String, protocol: String, shrimps: Shrimp, status: Option<String>, pond_id: Option<i32>, mode: Option<String>, empty_alarm: Option<bool>) -> Self {
        Self {
            id,
            network_id,
            name,
            address,
            protocol,
            pond_name: shrimps.0,
            farm_id: shrimps.1,
            status,
            pond_id,
            mode,
            empty_alarm
        }
    }

    pub fn update_mac_address(&self, mac: &str, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        info!("Actualizando mac address de {}", self.address);
        let result = conn.execute("UPDATE device set interface_mac_address=$1 where id=$2", &[&mac, &self.id]);
        match result {
            Ok(_r) => debug!("Actualizacion mac correcta"),
            Err(e) => info!("Error al actualizar la mac {}", e)
        }
    }
    /// Insert logs in device_status table, indicating if a device has a full hopper
    /// or a empty hopper.
    /// If error occurrs return a Postgres Error
    /// otherwise returns Ok
    pub fn create_device_status(&self, timestamp: &NaiveDateTime, status: DeviceStatus, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        // st tendra "empty" o "full"
        let st = match status {
            DeviceStatus::Empty => "empty",
            DeviceStatus::Full => "full"
        };

        info!("Insertando logs de tolva para {}", self.name);
        let create_date = chrono::Utc::now();
        let retained = !self.power(conn);
        // Insertamos los logs
        let result = conn.query("INSERT INTO device_status(create_date, device_id, timestamp, status, retained) \
    values($1, $2, $3, $4, $5)", &[&create_date.naive_utc(), &self.id, &timestamp, &st, &retained]);
        match result {
            Ok(_response) => {
                debug!("Devices status insertado");
            },
            Err(e) => {
                info!("Error devices status {}", e);
            }
        };

        // set as empty alarm
        let empty_alarm = match st {
            "empty" => true,
            "full" => false,
            &_ => false
        };

        let mut change_status: bool = false;
        let mut change_field: &str = "first_change_status_timestamp";
        let empty_alarm_device = match self.empty_alarm{
            Some(alarm) => alarm,
            _ => false
        };
        if !empty_alarm_device && empty_alarm{
            // device was full and now is empty
            change_status = true;
            change_field = "first_change_status_timestamp";
        }

        if empty_alarm_device && !empty_alarm{
            // device was empty and now is full
            change_status = true;
            change_field = "first_change_full_status_timestamp";
        }

        if change_status{
            let query = format!("UPDATE device set {change_field}=$1 where id=$2");
            let change_result = conn.execute(&query, &[&create_date.naive_utc(), &self.id]);
        }

        let result = conn.query("UPDATE device set hopper_status=$1, empty_alarm=$2 where id=$3",
                                &[&st, &empty_alarm, &self.id]);

        let device_alarm_history = create_or_update_device_alarm_history(Some(self.id),
                                                                             "empty_alarm".to_string(),
                                                                        empty_alarm, conn);
        match result {
            Ok(_response) => {
                debug!("Hopper status actualizado");
            },
            Err(e) => {
                info!("Error hopper status {}", e);
            }
        };
    }

    pub fn update_rssi_battery_timestamp(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>, timestamp: &NaiveDateTime, battery_info: String) {
        let statement = conn.prepare("UPDATE device set last_signal_date=$1, battery_level=$3 where id=$2").unwrap();
        let result = conn.execute(&statement, &[&timestamp, &self.id, &battery_info]);
        match result {
            Ok(_response) => {
                debug!("Last signal actualizado");
            },
            Err(e) => {
                info!("Error last_signal_date {}", e);
            }
        };
    }

    /// Update device last_comm information in every message arrived for this device
    /// If error occurrs return a Postgres Error
    /// otherwise returns Ok
    pub fn update_communication(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        let now = chrono::Utc::now();
        let now = now.naive_utc();
        debug!("Actualizando last_comm para {}", self.name);
        let statement = conn.prepare("UPDATE device set last_comm=$1 where id=$2").unwrap();
        let result = conn.execute(&statement, &[&now, &self.id]);
        match result {
            Ok(_response) => {
                debug!("Last comm actualizado");
            },
            Err(e) => {
                info!("Error last_comm {}", e);
            }
        };
    }

    pub fn create_logs_from_fetch(&self, var: &Var, value: &String, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        let now = chrono::Utc::now();
        let now = now.naive_utc();
        let val = value.parse::<f32>();
        let value = val.unwrap();
        // let statement = conn.prepare("SELECT * FROM base_var_fetch WHERE device_id=$1 and base_var_id=$2").unwrap();
        // let result = conn.query(&statement, &[&self.id, &var.base_var_id]);
        if let Some(code) = var.code.as_ref(){
            if code == "0D86"{
                info!("Si es 0D86");
                self.insert_status_log(&now, None, 0, Some(value), None, conn);
            }
        }
        // if let Ok(result) = result{
        //     if !result.is_empty() {
        //         self.insert_into_logs(var.base_var_id, &now, val.unwrap(), self.cycle_id, var.name.as_str(), None, conn);
        //     }
        // }
    }

    /// Update a child device setting parent_device to null
    pub fn update_parent_device(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        debug!("Actualizando parent_id para {}", self.name);
        let statement = conn.prepare("UPDATE device set uc_assigned_id=null where id=$1").unwrap();
        let result = conn.execute(&statement, &[&self.id]);
        match result {
            Ok(_response) => {
                debug!("Parent device id actualizado: UC {}", self.name);
            },
            Err(e) => {
                info!("Error parent device id {}", e);
            }
        };
    }

    /// Retrieve a variable with the required code
    /// Returns and option var
    /// Some if find a variable
    /// None if not find any variable with that code
    pub fn get_multiple_variables(&self, code: &str, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> Vec<Var> {
        info!("Obteniendo la variable {} para {}", code, self.name);
        let mut ilike_device = code.to_string();
        ilike_device.push_str(":%");
        let statement = conn.prepare("SELECT v.id, bv.size, bv.format, bv.name, bv.id, v.value, code from var v \
    inner join base_var bv on bv.id=v.base_var_id \
    where device_id=$1 and code ilike $2");
        let mut variables_vec: Vec<Var> = Vec::new();
        match statement {
            Ok(statement) => {
                info!("Ejecutando QUERY....");
                let result = conn.query(&statement, &[&self.id, &ilike_device]);
                let mut value: Option<String>;
                if let Ok(response) = result {
                    info!("Pushing variables....");
                    // Iteramos por cada registro encontrado y creamos el objeto device
                    for r in response {
                        value = r.get(5);
                        // Insertamos el objeto device en el vector
                        variables_vec.push(
                            Var::new(
                                r.get(0),
                                r.get(1),
                                r.get(2),
                                r.get(3),
                                r.get(4),
                                value.unwrap_or("".to_string()),
                                Some(r.get(6))
                            )
                        );
                    };
                }
            },
            Err(e) => info!("Error en GET_MULTIPLE {}", e)
        }
        // Error o no se encontraron registros para ese codigo de variable
        info!("Retornando variables....");
        variables_vec
    }

    /// Retrieve a variable with the required code
    /// Returns and option var
    /// Some if find a variable
    /// None if not find any variable with that code
    pub fn get_variable(&self, code: &str, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> Option<Var> {
        debug!("Obteniendo la variable {} para {}", code, self.name);
        let statement = conn.prepare("SELECT v.id, bv.size, bv.format, bv.name, bv.id, v.value, bv.code from var v \
    inner join base_var bv on bv.id=v.base_var_id \
    where device_id=$1 and code=$2").unwrap();
        let result = conn.query(&statement, &[&self.id, &code]);

        // Si no hay error en la consulta
        if let Ok(r) = result {

            // Si devolvio algun registro
            if !r.is_empty() {
                debug!("Se encontro la variable {} para {}", code, self.name);
                // Creamos el objeto variable
                let r = &r[0];
                let value: Option<String> = r.get(5);
                let variable: Var = Var::new(r.get(0), r.get(1),
                                             r.get(2), r.get(3),
                                             r.get(4), value.unwrap_or("".to_string()), r.get(6));

                // Retornamos some variable
                return Some(variable);
            }
        } else {
            info!("Ocurrio un error al obtener la variable {} {}", code, result.unwrap_err());
        }
        // Error o no se encontraron registros para ese codigo de variable
        None
    }

    pub fn get_sensors(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> Option<Vec<Device>> {
        let mut device_vec: Vec<Device> = Vec::new();
        let statement = conn.prepare("SELECT d.id, network_id, d.name, address, n.farm_id \
    from device d inner join network n on n.id=d.network_id inner join profile p on p.id=d.profile_id \
    where uc_assigned_id=$1 and p.profile_type='sensor' order by address asc").unwrap();
        let result = conn.query(&statement, &[&self.id]);
        debug!("Obteniendo los alimentadores...");
        if let Ok(response) = result {
            return if response.is_empty() {
                debug!("No hubo alimentadores...");
                None
            } else {
                // Iteramos por cada registro encontrado y creamos el objeto device
                for row in response {
                    // Insertamos el objeto device en el vector
                    device_vec.push(
                        Device::new(
                            row.get(0),
                            row.get(1),
                            row.get(2),
                            row.get(3),
                            self.protocol.clone(),
                            (None,
                             row.get(4)) as Shrimp,
                            None,
                            None,
                            None,
                            Some(false),
                        )
                    );
                };
                return Some(device_vec);
            }
        }
        None
    }

    pub fn get_feeders(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> Option<Vec<Device>> {
        let mut device_vec: Vec<Device> = Vec::new();
        let statement = conn.prepare("SELECT d.id, network_id, d.name, address, n.farm_id, d.empty_alarm \
    from device d inner join network n on n.id=d.network_id inner join profile p on p.id=d.profile_id \
    where uc_assigned_id=$1 and p.profile_type='feeder' order by address asc").unwrap();
        let result = conn.query(&statement, &[&self.id]);
        debug!("Obteniendo los alimentadores...");
        if let Ok(response) = result {
            return if response.is_empty() {
                debug!("No hubo alimentadores...");
                None
            } else {
                // Iteramos por cada registro encontrado y creamos el objeto device
                for row in response {
                    // Insertamos el objeto device en el vector
                    device_vec.push(
                        Device::new(
                            row.get(0),
                            row.get(1),
                            row.get(2),
                            row.get(3),
                            self.protocol.clone(),
                            (None,
                             row.get(4)) as Shrimp,
                            None,
                            None,
                            None,
                            row.get(5),
                        )
                    );
                };
                return Some(device_vec);
            }
        }
        None
    }

    /// Gets all child of the Central Unit(UC)
    /// Returns and Option vector of devices
    pub fn get_children_devices(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> Option<Vec<Device>> {
        let mut device_vec: Vec<Device> = Vec::new();
        let statement = conn.prepare("SELECT d.id, network_id, d.name, address, n.farm_id \
    from device d inner join network n on n.id=d.network_id \
    where uc_assigned_id=$1 order by address asc").unwrap();
        let result = conn.query(&statement, &[&self.id]);
        debug!("Obteniendo los alimentadores...");
        if let Ok(response) = result {
            return if response.is_empty() {
                debug!("No hubo alimentadores...");
                None
            } else {
                // Iteramos por cada registro encontrado y creamos el objeto device
                for row in response {
                    // Insertamos el objeto device en el vector
                    device_vec.push(
                        Device::new(
                            row.get(0),
                            row.get(1),
                            row.get(2),
                            row.get(3),
                            self.protocol.clone(),
                            (None,
                             row.get(4)) as Shrimp,
                            None,
                            None,
                            None,
                            Some(false),
                        )
                    );
                };
                return Some(device_vec);
            }
        } else {
            info!("ERROR AL OBTENER CHILDREN {}", result.unwrap_err())
        }
        None
    }

    /// Insert logs into database
    pub fn insert_into_logs(&self, base_var_id: i32, timestamp: &NaiveDateTime, value: f32, cycle_id: Option<i32>, code: &str, description: Option<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        info!("Insertando logs para {}", self.name);
        let value = value as f64;
        let create_date = chrono::Utc::now();
        // let statement = conn.prepare("INSERT INTO var_log(create_date, timestamp, device_id, base_var_id, value, cycle_id, code, description)\
        // values($1, $2, $3, $4, $5, $6, $7, $8)").unwrap();
        let result = conn.execute("INSERT INTO var_log(create_date, timestamp, device_id, base_var_id, value, cycle_id, code, description)\
    values($1, $2, $3, $4, $5, $6, $7, $8)", &[&create_date.naive_utc(), &timestamp, &self.id, &base_var_id, &value, &cycle_id, &code, &description]);
        match result {
            Ok(_response) => debug!("Logs guardado con exito"),
            Err(e) => info!("Error al guardar log {}", e)
        };
    }

    /// Insert status log into database
    pub fn insert_status_log(&self, timestamp: &NaiveDateTime, temperature: Option<i32>, signal: i32, battery: Option<f32>, panel: Option<f32>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        info!("Insertando status logs para {}", self.name);
        let create_date = chrono::Utc::now();
        let battery = battery.unwrap_or(0.0) as f64;
        let panel = panel.unwrap_or(0.0) as f64;
        let temperature = temperature.unwrap_or(0) as f64;
        let signal_float = -signal as f64;
        let mut signal_alarm = false;
        let signal = signal.to_string();
        let statement = conn.prepare("INSERT INTO device_log_status(create_date, device_id, timestamp, temp, signal, status_v_1, status_v_2, farm_id) VALUES($1, $2, $3, $4, $5, $6, $7, $8)").unwrap();
        let result = conn.execute(&statement,
                                  &[&create_date.naive_utc(), &self.id, &timestamp, &temperature, &signal_float, &battery, &panel, &self.farm_id]);
        let dev_statement = conn.prepare("UPDATE device set signal_alarm=$1, last_signal=$2 where id=$3").unwrap();
        match result {
            Ok(_response) => debug!("Status log guardado con exito"),
            Err(e) => info!("Error al guardar log {}", e)
        };
        // update signal alarm and value
        if signal_float <= -90.00 as f64 {
            signal_alarm = true;
        }
        let dev_result = conn.execute(&dev_statement,
                                      &[&signal_alarm, &signal_float, &self.id]);
        let device_alarm_history = create_or_update_device_alarm_history(Some(self.id),
                                                                             "signal_alarm".to_string(),
                                                                        signal_alarm, conn);

        match dev_result {
            Ok(_response) => debug!("Signal actualizado con exito"),
            Err(e) => info!("Error al actualizar signal {}", e)
        }
    }

    pub fn get_or_create_hydro_now(&self, date: NaiveDateTime, now: NaiveDateTime, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> Result<HydrophoneAnalysis, Error> {
        let result = conn.query("SELECT id FROM hydrophone_analysis where create_date<=$1 and create_date>=$2 and device_id=$3", &[&now, &date, &self.id])?;
        return if !result.is_empty() {
            let r = &result[0];
            let hydro = HydrophoneAnalysis {
                id: r.get(0),
                device_id: self.id
            };
            Ok(hydro)
        } else {
            conn.execute("INSERT INTO hydrophone_analysis(device_id, create_date, farm_id, pond_id) VALUES($1, $2, $3, $4)", &[&self.id, &now, &self.farm_id, &self.pond_id]);
            let hydro = self.get_or_create_hydro_now(date, now, conn);
            hydro
        }
    }

    pub fn insert_alarm(&self, data: i32, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        let alarms: HashMap<i32, String> = HashMap::from([
            (1, "Salida 1 desconectado (motor 1)".to_string()),
            // (2, "Salida 2 desconectado (motor 2)".to_string()),
            (4, "Salida 1 excedio maximo consumo".to_string()),
            // (5, "Salida 2 excedio maximo consumo".to_string()),
            (20, "Bateria baja".to_string()),
        ]);
        let create_date = chrono::Utc::now();
        let statement_insert = conn.prepare("INSERT INTO alarm_alarm(create_date, device_id, pond, bit_position, incident_number, name, farm_id, timestamp, active) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true)").unwrap();
        let statement_update = conn.prepare("UPDATE alarm_alarm set active=False \
            where device_id=$1 and active=True and bit_position=$2").unwrap();
        for (i, name) in alarms.iter() {
            let result: Result<u64, Error>;
            if data & i32::pow(2, *i as u32) > 0 {
                result = conn.execute(&statement_insert, &[&create_date.naive_utc(), &self.id, &self.pond_name, &i, &1, name, &self.farm_id, &create_date.naive_utc()]);
            } else {
                result = conn.execute(&statement_update, &[&self.id, i]);
            }

            match result {
                Ok(_response) => debug!("Alarma insertada correctamente"),
                Err(e) => info!("Error al insertar alarma {}", e)
            };
        }
    }

    /// Indicates if an unit-central has an active hydrophone
    pub fn has_hydro(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> bool {
        let active_hydrophone = self.get_variable("0C03:13", conn);
        if let Some(hydro) = active_hydrophone {
            if hydro.value.as_str() == "1" {
                return true;
            }
        }
        false
    }

    /// Insert a pre alarm, this is for UC1.0 en hydrophone mode
    /// when log 30 arrives create a new pre alarm
    /// server should process the pre alarm and send a message to UC
    pub fn insert_pre_alarm(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        let result_select = conn.query("SELECT id from alarm_pre_alarm where device_id=$1", &[&self.id]);
        if let Ok(result) = result_select {
            if !result.is_empty() {
                return ();
            }
        }
        let prepare_insert = conn.prepare("INSERT INTO alarm_pre_alarm(create_date, device_id) \
            VALUES ($1, $2)").unwrap();
        let create_date = chrono::Utc::now();
        let result = conn.query(&prepare_insert, &[&create_date.naive_utc(), &self.id]);
        match result {
            Ok(response) => debug!("Pre alarma insertada"),
            Err(error) => info!("Error al insertar pre alarma {}", error)
        }
    }

    /// Delete alarms when a device stop send it
    // pub fn clean_alarms(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>){
    //     let create_date = chrono::Utc::now() - chrono::Duration::minutes(3);
    //     let has_active_alarm = conn.query("SELECT id from alarm_alarm where timestamp >= $1 and device_id=$2 and active=true limit 1",
    //                                       &[&create_date.naive_utc(), &self.id]);
    //     if let Ok(result) = has_active_alarm{
    //         if result.is_empty(){
    //             info!("Eliminando alarmas para {}", self.name);
    //             conn.execute("DELETE FROM alarm_alarm where device_id=$1", &[&self.id]);
    //         }
    //     }
    //     conn.execute("DELETE FROM alarm_pre_alarm where device_id=$1", &[&self.id]);
    // }

    pub fn update_status(&self, status: &str, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        let result = conn.execute("UPDATE device set status=$1, responded_message_counter=responded_message_counter+1, status_hidden='ok' where id=$2", &[&status, &self.id]);
        match result {
            Ok(_r) => debug!("Status actualizado"),
            Err(r) => info!("Error al actualizar estado {}: {}", self.address, r)
        }
    }

    /// Get if device is power off or power on
    pub fn power(&self, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> bool {
        let bytes_var = self.get_variable("0C03:15", conn);
        if let Some(byte_var) = bytes_var {
            if byte_var.value == "1".to_string() {
                return true;
            }
            return false;
        }
        true
    }

    pub fn verify_bytes_seteo(&self, code: &str, value: &str, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
        if code == "0C03" {
            let variables = self.get_multiple_variables(code, conn);
            if !variables.is_empty() {
                for var in variables {
                    let code = var.code.as_ref().unwrap();
                    let sp_code: Vec<&str> = code.split(':').collect();
                    let current_value = var.value.parse::<i32>();
                    let current_value = match current_value {
                        Ok(v) => v as i64,
                        Err(e) => 0 as i64
                    };
                    let mut after_value = 2i64.pow(sp_code[1].parse::<u32>().unwrap()) & str_to_int64(value, 16);
                    after_value = match after_value {
                        0 => 0,
                        _ => 1
                    };
                    info!("Variables actual {}, after {} - {}", var.code.as_ref().unwrap(), &after_value, self.name);
                    if current_value != after_value {
                        var.update(&after_value.to_string(), conn);
                    }
                }
            }
        }
    }
}


/// Returns a device (if exists), for this case should be an UC device
pub fn get_device(address: String, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> Result<Option<Device>, Error> {
    let result = conn.query("SELECT d.id, d.network_id, d.name, address, p.model, sp.name, sf.id, d.status, sp.id, d.mode, d.empty_alarm \
from device d \
left join protocol p on p.id=d.protocol_id \
left join shrimps_pond sp on sp.id=d.pond_id \
left join shrimps_farm sf on sf.id=sp.farm_id
where address=$1", &[&address])?;
    if result.len() == 1 {
        debug!("Se encontro el dispositivo con address {}", address);
        let result = &result[0];
        // Creamos el dispositivo encontrado
        let device = Device::new(result.get(0), result.get(1),
                                 result.get(2), result.get(3), result.get(4),
                                 (result.get(5), result.get(6)) as Shrimp,
                                 result.get(7), result.get(8), result.get(9), result.get(10));
        Ok(Some(device))
    } else {
        info!("No se encontro el dispositivo con address {}", address);
        Ok(None)
    }
}

pub fn bulk_update_communication(devices: &Vec<i32>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
    let devices_str: Vec<String> = devices
        .iter()
        .map(|x| x.to_string())
        .collect();

    let now = chrono::Utc::now();
    let now = now.naive_utc();
    let query = format!("UPDATE device set last_comm='{}' where id in ({})", now, devices_str.join(","));
    let result = conn.batch_execute(&query);
    match result {
        Ok(_r) => debug!("Dispostivos actualizados"),
        Err(r) => info!("Error al bulk update {}", r)
    }
}

pub fn bulk_clean_alarms(devices: &Vec<i32>, parent_device: Option<i32>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
    let devices_str: Vec<String> = devices
        .iter()
        .map(|x| x.to_string())
        .collect();

    let create_date = chrono::Utc::now() - chrono::Duration::minutes(3);
    let create_date = create_date.naive_utc();
    let query = format!("DELETE FROM alarm_alarm where device_id in ({}) and timestamp <= '{}' \
    and active=true", devices_str.join(","), create_date);
    let result = conn.batch_execute(&query);

    // alarm queries
    let alarm_query = conn.prepare("SELECT * from alarm_alarm where \
    device_id=$1 and active = true and bit_position=$2 limit 1", ).unwrap();
    let bat_alarm_dev_update = conn.prepare("UPDATE device set \
    battery_alarm=$1 where id=$2").unwrap();
    let motor_alarm_dev_update = conn.prepare("UPDATE device set \
    motor_alarm=$1 where id=$2").unwrap();

    match result {
        Ok(_r) => debug!("Alarm alarm eliminados"),
        Err(r) => info!("Error al bulk alarm_alarm {}", r)
    }

    if let Some(parent_device) = parent_device {
        let result = conn.execute("DELETE FROM alarm_pre_alarm where device_id=$1", &[&parent_device]);
        match result {
            Ok(_r) => debug!("Alarm pre alarm eliminados"),
            Err(r) => info!("Error al bulk alarm_pre {}", r)
        }
    }
    // check alarms for each device
    for dev in devices_str {
        let dev_id = dev.parse::<i32>().unwrap();
        let mut set_alarm: bool = false;
        // battery
        let has_active_bat_alarm = conn.query(&alarm_query, &[&dev_id, &20]);
        if let Ok(bat_active_alarm_result) = has_active_bat_alarm {
            if bat_active_alarm_result.is_empty() {
                set_alarm = false;
                conn.execute(&bat_alarm_dev_update, &[&set_alarm, &dev_id]);
            } else {
                set_alarm = true;
                conn.execute(&bat_alarm_dev_update, &[&set_alarm, &dev_id]);
            }
            let device_alarm_history = create_or_update_device_alarm_history(Some(dev_id),
                                                                             "battery_alarm".to_string(),
                                                                        set_alarm, conn);
            info!("Editando alarma de batería para {}", dev);
        }
        // motor
        let has_active_motor_alarm = conn.query(&alarm_query, &[&dev_id, &1]);
        if let Ok(motor_active_alarm_result) = has_active_motor_alarm {
            if motor_active_alarm_result.is_empty() {
                set_alarm = false;
                conn.execute(&motor_alarm_dev_update, &[&set_alarm, &dev_id]);
            } else {
                set_alarm = true;
                conn.execute(&motor_alarm_dev_update, &[&set_alarm, &dev_id]);
            }
            let device_alarm_history = create_or_update_device_alarm_history(Some(dev_id),
                                                                             "motor_alarm".to_string(),
                                                                        set_alarm, conn);
            info!("Editando alarma de motor para {}", dev);
        }
    }
}

pub fn create_or_update_device_alarm_history(device_id: Option<i32>, alarm_type: String, alarmed: bool,
                                             conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) {
    let now = chrono::Utc::now();
    let now = now.naive_utc();

    let query = format!("SELECT id FROM device_alarm_history \
        where {alarm_type}=True and still_alarmed=True and device_id=$1");
    let result = conn.query(&query, &[&device_id]).unwrap();

    // device alarm history query
    let create_query = format!("INSERT INTO device_alarm_history(device_id, {alarm_type}, \
                    still_alarmed, alarm_counter, last_alarmed_date, create_date) \
                    VALUES ($1, $2, $3, $4, $5, $6)");
    return if !result.is_empty() {
        let r = &result[0];
        let dev_alarm_history = DeviceAlarmHistory {
            id: r.get(0)
        };
        // update existing record
        if alarmed {
            let history_alarm_record_update = conn.execute(
            "UPDATE device_alarm_history SET alarm_counter=alarm_counter+1, last_alarmed_date=$1 \
                where id=$2", &[&now, &dev_alarm_history.id]);

        } else {
            let history_alarm_record_update = conn.execute(
            "UPDATE device_alarm_history SET still_alarmed=False, non_alarmed_date=$1 \
                where id=$2", &[&now, &dev_alarm_history.id]);

        }
        // match history_alarm_record_update {
        //         Ok(_r) => debug!("Editado device alarm history"),
        //         Err(r) => info!("Error al editar device alarm history {}", r)
        //     }

    } else {
        // create new record
        if alarmed {
            let history_alarm_record_create = conn.execute(&create_query,
                &[&device_id, &true, &true, &1, &now, &now]);

            match history_alarm_record_create {
                Ok(_r) => debug!("Creado device alarm history"),
                Err(r) => info!("Error al crear device alarm history {}", r)
            }
        }

    }
}
