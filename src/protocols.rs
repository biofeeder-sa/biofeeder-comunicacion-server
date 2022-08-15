use std::collections::HashMap;
use std::process;
use chrono::{Datelike, Timelike};
use to_binary::BinaryString;
use log::{error, info, debug};
use r2d2::PooledConnection;
use r2d2_postgres::PostgresConnectionManager;
use r2d2::Pool;
use postgres::NoTls;
use crate::device::{Device, get_device, DeviceStatus, bulk_update_communication, bulk_clean_alarms};
use crate::message::MessageMode;
use crate::vars::{Var, str_to_int, ustr_to_int};

type ResponseCommand = (Option<i32>, MessageMode, String, Option<HashMap<String, String>>);


/// Trait Protocol, other protocols should implement this
pub trait Protocol{

    /// Process al packets response packets or logs packets from devices
    fn process_packet<'a>(&self, device: &Device, data: &Vec<&'a str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>){
        // Sacamos el comando
        let command = data[0];
        let mut real_data: Vec<&str>;
        if data.len() > 3 {
            // Quitamos el message ID
            real_data = data[3..].to_vec();
        }else{
            real_data = data.clone();
        }

        match command {
            // "23" => self.read_response_function(device, &mut real_data, conn),
            "30" => self.feed_log_function(device, &real_data, conn),
            // "31" => self.read_response_vars_other_function(device, &real_data, conn),
            // "32" => self.write_response_vars_other_function(device, &real_data, conn),
            // "33" => self.feed_log_confirmation_function(device, &real_data, conn),
            // "37" => self.log_accumulation_day_function(device, &real_data, conn),
            // "40" => self.rssi_function(device, &real_data, conn),
            // "44" => self.log_sound_function_x2(device, &real_data, conn),
            // "50" => self.log_sensor_do_temp_function(device, &real_data, conn),
            "52" => self.log_alarms_function(device, &real_data, conn),
            "80" => self.log_status_device_function(device, &real_data, conn),
            // "81" => self.log_status_uc_function(device, &real_data, conn),
            "82" => self.log_sound_function(device, &real_data, conn),
            // "83" => self.log_dosage_hydro_function(device, &real_data, conn),
            // "85" => self.log_recovery_function(device, &data[1..].to_vec(), conn),
            // "88" => self.assign_device_response_function(device, &real_data, conn),
            "98" => self.mac_address_response_function(device, &real_data, conn),
            "99" => self.ping_function(device, &real_data, conn),
            // "4E" => self.hydrophone_error_function(),
            // "4F" => self.xbee_error_function(),
            // "DD" => self.log_kg_hopper(device, &real_data, conn),

            _ => self.ack_response_function()
        };

        // Obtenemos la respuesta de la funcion que proceso el mensaje
        // let (child_device_id, mode, note, placeholder) = response;
        //
        // if let MessageMode::LogRecovery = mode{
        //     return ;
        // };
        //
        // let now = chrono::Utc::now().naive_utc();
        // let statement_error = conn.prepare("INSERT into message(create_date, raw_frame, device_id, \
        // address_dst, comm_type, network_id, mode, note, status, create_uid) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)").unwrap();
        // if let MessageMode::HydrophoneError = mode{
        //     let _result = conn.execute(&statement_error,
        //              &[&now, &"4E", &device.id, &device.address, &"rx", &device.network_id, &"rr", &note, &"received"]);
        //     return ;
        // }
        //
        // if let MessageMode::XbeeError = mode{
        //     let _result = conn.execute(&statement_error,
        //              &[&now, &"4F", &device.id, &device.address, &"rx", &device.network_id, &"rr", &note, &"received"]);
        //     return ;
        // }
        //
        // // Sacamos el Message ID
        // let frame_id = data[1..3].join("");
        //
        // // Se convierte a un i32, de hexadecimal a entero base 10
        // let frame_id = i32::from_str_radix(frame_id.as_str(), 16).unwrap();
        //
        // // Hora actual del servidor(UTC)
        //
        // let before = now - chrono::Duration::minutes(30);
        // let statement = conn.prepare("SELECT id from message \
        // WHERE frame_id=$1 and device_id=$2 and create_date>=$3 and comm_type='tx' order by create_date desc").unwrap();
        // // Buscamos el mensaje padre
        // let message = conn.query(&statement, &[&frame_id, &device.id, &before]);
        // let mut parent_id: Option<i32> = None;
        // match message{
        //     Ok(m) => {
        //         if !m.is_empty(){
        //             let m = &m[0];
        //             let message_id = m.get(0);
        //             parent_id = Some(message_id);
        //             let update_statement = conn.prepare("UPDATE message set status='responded' WHERE id=$1").unwrap();
        //             let result = conn.execute(&update_statement, &[&message_id]);
        //             match result{
        //                 Ok(_response) => (),
        //                 Err(e) => info!("Error when update status message {}", e)
        //             };
        //         }
        //     },
        //     Err(e) => info!("Error al consultar mensaje padre {}", e)
        // }
        //
        //
        // // Representacion del modo de respuesta del mensaje
        // let response_mode = match mode {
        //     MessageMode::ModeWriteResponse => "wr",
        //     MessageMode::ModeLogResponse => "lr",
        //     MessageMode::ModeReadResponse => "rr",
        //     _ => ""
        // };
        // let mut placeholder_string: String = String::new();
        //
        // // Se transforma HashMap a un String
        // if let Some(value) = placeholder{
        //
        //     // Inicializamos un vector de Strings
        //     let mut my_vec: Vec<String> = Vec::new();
        //     for (key, v) in value.iter(){
        //         my_vec.push(format!("{}: {}", key, v));
        //     }
        //
        //     // Transformamos en un String el vector
        //     let pp = my_vec.join(", ");
        //     placeholder_string = format!("{{{}}}", pp);
        // };
        //
        // // Transformamos en un String el vector de los datos
        // let msg = data.join(" ");
        // let insert_statement = conn.prepare("INSERT into message(create_date, raw_frame, frame_id, placeholder, device_id, \
        // address_dst, comm_type, device_child_id, network_id, parent_id, mode, note, status, create_uid) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 1)").unwrap();
        // let result = conn.execute(&insert_statement,
        //              &[&now, &msg, &frame_id, &placeholder_string, &device.id, &device.address,
        //                  &"rx", &child_device_id, &device.network_id, &parent_id, &response_mode, &note, &"received"]);
        //
        // match result{
        //     Ok(_response) => {
        //         info!("Mensaje creado");
        //     },
        //     Err(e) => {
        //         info!("Error al crear mensaje {}", e);
        //     }
        // };
    }

    /// Parse a read response from device
    fn read_response_function(&self, device: &Device, data: &mut Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        // Option placeholder
        let mut placeholder: Option<HashMap<String, String>> = None;
        data.remove(0);
        // Tooltip, despues usado para placeholder
        let mut hash_tooltip: HashMap<String, String> = HashMap::new();

        while !data.is_empty(){
            // Obtenemos el codigo de la variable
            let code = data[..2].join("");
            // Obtenemos el objeto variable
            let result_var = device.get_variable(code.as_str(), conn);

            // Si existe la variable
            if let Some(var) = result_var{
                let var_len = var.size as usize;
                // Obtenemos el valor para esa variable
                let real_data = data[2..=1 + var_len].join(" ");

                // Decodificados el valor en hexadecimal
                let decoded_data = var.decode(real_data.as_str());

                // Verificamos y actualizamos los bytes de seteo en caso de haya diferencias
                device.verify_bytes_seteo(code.as_str(), &decoded_data, conn);
                if code.as_str() == "0C03" {
                    info!("Sali de bytes seteo");
                }
                // Hacemos UPDATE a la base de datos de esa variable
                var.update(&decoded_data, conn);
                if code.as_str() == "0C03" {
                    info!("Sali de update");
                }
                device.create_logs_from_fetch(&var, &decoded_data, conn);
                hash_tooltip.insert(var.name, decoded_data);

                // Cortamos la data
                if 1 + var_len > data.len(){
                    break;
                }

                data.drain(..=1 + var_len);
            }else{
                // Si no se encontro variable termina el bucle, no es posible seguir
                // parseando la trama
                break;
            }

        }
        if !hash_tooltip.is_empty(){
            placeholder = Some(hash_tooltip);
        }
        (None, MessageMode::ModeReadResponse, "Respuesta de Lectura".to_string(), placeholder)
    }

    fn ack_response_function(&self) -> ResponseCommand{
        (None, MessageMode::ModeWriteResponse, "Respuesta de escritura".to_string(), None)
    }

    fn log_recovery_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn rssi_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn read_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn write_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_kg_hopper(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_alarms_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_accumulation_day_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_sensor_do_temp_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn feed_log_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn feed_log_confirmation_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_sound_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_sound_function_x2(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_dosage_hydro_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_status_uc_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn log_status_device_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn xbee_error_function(&self) -> ResponseCommand;

    fn hydrophone_error_function(&self) -> ResponseCommand;

    fn assign_device_response_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn mac_address_response_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;

    fn ping_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand;
}

/// Struct for communication through X2
struct ProtocolX2;

impl Protocol for ProtocolX2{
    fn log_recovery_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand{
        todo!()
    }

    fn rssi_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn read_response_vars_other_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn write_response_vars_other_function(&self, _device: &Device, _data: &Vec<&str>, _conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_kg_hopper(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand{
        todo!()
    }

    fn log_alarms_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_accumulation_day_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_sensor_do_temp_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn feed_log_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn feed_log_confirmation_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_sound_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_sound_function_x2(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_dosage_hydro_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_status_uc_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_status_device_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn xbee_error_function(&self) -> ResponseCommand {
        todo!()
    }

    fn hydrophone_error_function(&self) -> ResponseCommand {
        todo!()
    }

    fn assign_device_response_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn mac_address_response_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn ping_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }
}

/// Struct for communication through a UC 1.0
struct ProtocolUC;

// impl Protocol for ProtocolUC{
//     fn log_recovery_function(&self, _device: &Device, _data: &Vec<&str>, _conn: &mut  PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand{
//         todo!()
//     }
//
//     /// Save rssi logs into database
//     fn rssi_function(&self, device: &Device, data: &Vec<&str>, conn: &mut  PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         // Obtenemos todos los hijos de esa UC
//         let devices = device.get_children_devices(conn);
//         let mut placeholder: Option<HashMap<String, String>> = None;
//
//         // Si existe algun hijo(alimentador)
//         if let Some(d) = devices{
//             let mut hash_tooltip: HashMap<String, String> = HashMap::new();
//             let data_len: usize = data.len();
//             let now = chrono::Utc::now();
//             let now = now.naive_utc();
//             // let now = now.to_string();
//
//             // Iteramos sobre los datos
//             // En esta trama llegan para todos los alimentadores
//             for i in 0..data_len{
//                 // Segun la posicion sacamos el alimentador asociado
//                 let device_id = &d[i];
//
//                 // Obtenemos el valor para ese alimentador
//                 let val = i32::from_str_radix(data[i], 16).unwrap() as f32;
//                 let var_id = device_id.get_variable("4442", conn);
//                 if let Some(var) = var_id{
//
//                     // Actualizamos la variable RSSI
//                     var.update(&val.to_string(), conn);
//                     hash_tooltip.insert(device_id.name.to_string(), val.to_string());
//
//                     // Insertamos el log en la base de datos
//                     device_id.insert_into_logs(var.base_var_id, &now, val, device.cycle_id, "4442", None, conn);
//                 }
//             }
//             if !hash_tooltip.is_empty(){
//                 placeholder = Some(hash_tooltip);
//             }
//         }
//         (None, MessageMode::ModeLogResponse, "Log RSSI".to_string(), placeholder)
//
//     }
//
//     /// Parse a read response from feeders
//     fn read_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         // Sacamos la posicion del alimentador que respondio
//         let position = str_to_int(data[0], 16);
//
//         // Tamano de la trama
//         let variables_len = str_to_int(data[1], 16);
//
//         // Los datos que deben ser procesados, esto es, quitar la posicion y el tamano de la trama
//         let mut raw_data = data[2..data.len()].to_vec();
//         let mut placeholder: HashMap<String, String> = HashMap::new();
//
//         // El codigo va a cambiar
//         let mut code: String;
//
//         // Todos los alimentadores asociados a esta UC
//         let op_children = device.get_children_devices(conn);
//         if let Some(children) = op_children{
//             // Obtenemos el alimentador
//             let child = &children[position as usize];
//             let mut var: Option<Var>;
//             let mut len_data_var = 0;
//             let mut devices_vec: Vec<i32> = Vec::new();
//             // Iteramos sobre las variables respondidas
//             for _i in 0..variables_len {
//
//                 // El codigo de la variable
//                 code = raw_data[..2].join("");
//                 var = child.get_variable(code.as_str(), conn);
//                 devices_vec.push(child.id);
//                 // child.update_communication(conn);
//                 if let Some(v) = var{
//                     len_data_var = v.size as usize;
//                     let decoded_value = v.decode(&raw_data[2..=1 + len_data_var].join(" "));
//                     child.verify_bytes_seteo(code.as_str(), &decoded_value, conn);
//                     child.create_logs_from_fetch(&v, &decoded_value, conn);
//                     v.update(&decoded_value, conn);
//                     placeholder.insert(v.name, decoded_value);
//                 }
//                 raw_data.drain(..=1 + len_data_var);
//             };
//             bulk_update_communication(&devices_vec, conn);
//             placeholder.insert("Dispositivo".to_string(), child.name.clone());
//         }
//
//         (None, MessageMode::ModeReadResponse, "Respuesta lectura alimentador".to_string(), Some(placeholder))
//     }
//
//     /// Simple ACK is returned
//     fn write_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         let op_children = device.get_children_devices(conn);
//         let position = str_to_int(data[0], 16);
//         let mut tooltip: Option<HashMap<String, String>> = None;
//         if let Some(children) = op_children {
//             // Obtenemos el alimentador
//             let child = &children[position as usize];
//             let placeholder: HashMap<String, String> = HashMap::from([
//                 ("Dispositivo".to_string(), child.name.clone())
//             ]);
//             tooltip = Some(placeholder);
//         };
//         (None, MessageMode::ModeWriteResponse, "Respuesta de escritura alimentador".to_string(), tooltip)
//     }
//
//     fn log_kg_hopper(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand{
//         todo!()
//     }
//
//     fn log_alarms_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         let device_position = str_to_int(data[6], 16);
//         let alarms = str_to_int(data[7..11].join("").as_str(), 16);
//         let devices = device.get_children_devices(conn).unwrap();
//         let target_device_id = &devices[device_position as usize];
//         target_device_id.insert_alarm(alarms, conn);
//         (None, MessageMode::ModeLogResponse, "Log de alarma".to_string(), None)
//     }
//
//     /// Parse a accumulation log from UC
//     fn log_accumulation_day_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         // Sacamos el ano actual para ser usado al guardar el log
//         // esto debido a un problema de las UC 1.0 que envian con ano incorrecto
//         let year = (chrono::offset::Utc::now().year() % 2000).to_string();
//         let mut placeholder: HashMap<String, String> = HashMap::new();
//
//         // Obtenemos la fecha y hora que nos envia la UC, obviamos el ano
//         let hex_date = data[0..5].to_vec();
//
//         // Transformas el ano de hexadecimal a decimal
//         let mut real_date: Vec<String> = hex_date
//             .iter()
//             .map(|x| str_to_int(*x, 16).to_string())
//             .collect();
//         real_date.push(year);
//
//         // Obtenemos el objeto NaiveDateTime para despues formatearlo
//         let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
//         // Timestamp formatado
//         // let timestamp = timestamp.to_string();
//
//         // Obtenemos el valor de los gramos acumulados, de hexadecimal a decimal
//         let value = str_to_int(data[6..10].to_vec().join("").as_str(), 16) as f32;
//
//         // Actualizamos la variable de gramos acumulados
//         let var_id = device.get_variable("0E72", conn);
//
//         if let Some(var) = var_id {
//             placeholder.insert(var.name, value.to_string());
//             // TODO: Es realmente necesario crear estos logs?
//             // Creamos logs de los gramos acumulados por dia
//             device.insert_into_logs(var.base_var_id, &timestamp, value, device.cycle_id, "0E72",None, conn);
//         }
//         (None, MessageMode::ModeLogResponse, "Log de acumulado x dia".to_string(), Some(placeholder))
//     }
//
//     /// Parse a oxygen/temperatura frame
//     fn log_sensor_do_temp_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         let sensor_type = device.get_variable("AC00", conn);
//         let year = (chrono::offset::Utc::now().year() % 2000).to_string();
//         let mut placeholder: HashMap<String, String> = HashMap::new();
//         let hex_date = data[0..5].to_vec();
//         let mut real_date: Vec<String> = hex_date
//             .iter()
//             .map(|x| str_to_int(*x, 16).to_string())
//             .collect();
//         real_date.push(year);
//         let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
//
//         // Transformamos el valor de la temperatura de hexadecimal a decimal
//         let value = ustr_to_int(data[6..10].join("").as_str(), 16);
//
//         // Convertimos en un array de bytes u8
//         let gg: [u8; 4] = value.to_be_bytes();
//
//         // Transformamos de [u8] a flotante f32
//         let value = f32::from_ne_bytes(gg);
//
//         // Obtenemos la variable y creamos el log
//         let var_id = device.get_variable("0E87", conn);
//
//         if let Some(var) = var_id {
//             placeholder.insert(var.name, value.to_string());
//
//             device.insert_into_logs(var.base_var_id, &timestamp, value, device.cycle_id, "0E87", None, conn);
//         }
//
//         // Transformamos de [u8] a flotante f32
//         let mut value: f32;
//
//
//         if let Some(sensor_type) = sensor_type{
//             // Obtenemos la variable y creamos el log
//             let var_id = device.get_variable("0E8B", conn);
//
//             let mut value_u32: u32 = 0;
//
//             if sensor_type.value == "4".to_string() || sensor_type.value == "04".to_string(){
//                 // Transformamos el valor del oxigeno de hexadecimal a decimal
//                 value_u32 = ustr_to_int(data[14..=17].join("").as_str(), 16);
//
//                 // Convertimos en un array de bytes u8
//                 let gg: [u8; 4] = value_u32.to_be_bytes();
//
//                 // Transformamos de [u8] a flotante f32
//                 value = f32::from_ne_bytes(gg);
//
//             }else{
//                 // Transformamos el valor del oxigeno de hexadecimal a decimal
//                 value_u32 = ustr_to_int(data[10..14].join("").as_str(), 16);
//
//                 let gg: [u8; 4] = value_u32.to_be_bytes();
//
//                 // Transformamos de [u8] a flotante f32
//                 value = f32::from_ne_bytes(gg);
//
//                 if device.address.as_str() == "0013A20041646DB3"{
//                     value *= 13.04f32
//                 }
//             }
//
//
//             if let Some(var) = var_id {
//                 placeholder.insert(var.name, value.to_string());
//
//                 device.insert_into_logs(var.base_var_id, &timestamp, value, device.cycle_id, "0E8B",None, conn);
//             }
//
//         }
//         let sensors = device.get_sensors(conn);
//         if let Some(sensors) = sensors{
//             for sensor in sensors{
//                 sensor.update_communication(conn);
//             }
//         }
//
//         (None, MessageMode::ModeLogResponse, "Log de Do/Temp".to_string(), Some(placeholder))
//     }
//
//     /// Parse a feed log from UC
//     fn feed_log_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         let year = (chrono::offset::Utc::now().year() % 2000).to_string();
//         let mut placeholder: HashMap<String, String> = HashMap::new();
//         let hex_date = data[0..5].to_vec();
//         let mut real_date: Vec<String> = hex_date
//             .iter()
//             .map(|x| str_to_int(*x, 16).to_string())
//             .collect();
//         real_date.push(year);
//         let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
//
//         // Obtenemos el valor de hexadecimal a decimal y lo transformamos en f32
//         let value = str_to_int(data[6..10].to_vec().join("").as_str(), 16) as f32;
//         let var_id = device.get_variable("0D49", conn);
//
//         if let Some(var) = var_id {
//             placeholder.insert(var.name, value.to_string());
//
//             device.insert_into_logs(var.base_var_id, &timestamp, value,device.cycle_id, "0D49",None, conn);
//         }
//
//         // Obtenemos la data de los estados de dispositivos
//         let empty_feeders = str_to_int(data[14..18].to_vec().join("").as_str(), 16);
//         let response_feeders = str_to_int(data[10..14].to_vec().join("").as_str(), 16);
//         let children = device.get_feeders(conn).unwrap();
//
//         let base: i32 = 2;
//         let mut devices_vec: Vec<i32> = Vec::new();
//         // Iteramos sobre todos los alimentadores
//         for (i, child) in children.iter().enumerate(){
//
//             // Si esta vacio
//             if empty_feeders & base.pow(i as u32) > 0 || response_feeders & base.pow(i as u32) == 0{
//                 child.create_device_status(&timestamp, DeviceStatus::Empty, conn);
//             }else if response_feeders & base.pow(i as u32) > 0{
//                 child.create_device_status(&timestamp, DeviceStatus::Full, conn);
//             }
//             devices_vec.push(child.id);
//         }
//         bulk_update_communication(&devices_vec, conn);
//
//         devices_vec.push(device.id);
//         bulk_clean_alarms(&devices_vec, Some(device.id), conn);
//
//         (None, MessageMode::ModeLogResponse, "Log acumulado x hora".to_string(), Some(placeholder))
//     }
//
//     /// Parse a confirmation feed log
//     fn feed_log_confirmation_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         let year = (chrono::offset::Utc::now().year() % 2000).to_string();
//         let mut placeholder: HashMap<String, String> = HashMap::new();
//         let hex_date = data[0..5].to_vec();
//         // hex_date.push(year.as_str());
//         let mut real_date: Vec<String> = hex_date
//             .iter()
//             .map(|x| str_to_int(*x, 16).to_string())
//             .collect();
//         real_date.push(year);
//         let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
//
//         // Sacamos el valor de los gramos por ciclo
//         let value = str_to_int(data[7..9].to_vec().join("").as_str(), 16) as f32;
//
//         // Sacamos la variable de los gramos por ciclo y creamos el log
//         let var_id = device.get_variable("0CEB", conn);
//
//         if let Some(var) = var_id {
//             placeholder.insert(var.name, value.to_string());
//
//             device.insert_into_logs(var.base_var_id, &timestamp, value,device.cycle_id, "0CEB",None, conn);
//         }
//
//         if device.has_hydro(conn){
//             device.insert_pre_alarm(conn);
//         }
//
//         (None, MessageMode::ModeLogResponse, "Log de alimentacion".to_string(), Some(placeholder))
//
//     }
//
//     fn log_sound_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         todo!()
//     }
//
//     fn log_sound_function_x2(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         let raw_data = &data[6..data.len()];
//         let date: Vec<&str> = data[..6].to_vec();
//         let var = device.get_variable("DDE0", conn).unwrap();
//         let var_interval_sound = device.get_variable("DDDE", conn).unwrap();
//
//         // Obtenemos la variable indicador de duracion de sonido
//         let total_seconds = var_interval_sound.value.as_str().parse::<i64>().unwrap();
//         let real_date: Vec<String> = date
//             .iter()
//             .map(|x| str_to_int(*x, 16).to_string())
//             .collect();
//         let mut timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
//         let interval_sec = 10i64;
//         // Iteramos sobre la data y obtenemos cada valor de sonido
//         for sound_log in raw_data.chunks(2){
//             // Sacamos el valor como f32
//             let value = str_to_int(sound_log[..=1].to_vec().join("").as_str(), 16) as f32;
//             debug!("Insertando sonido {}...", value);
//             device.insert_into_logs(var.base_var_id, &timestamp, value, device.cycle_id, "DDE0", None, conn);
//
//             timestamp += chrono::Duration::seconds(interval_sec);
//         };
//         (None, MessageMode::ModeLogResponse, "Log de sonido".to_string(), None)
//     }
//
//     fn log_dosage_hydro_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         todo!()
//     }
//
//     fn log_status_uc_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         todo!()
//     }
//
//     fn log_status_device_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         todo!()
//     }
//
//     fn xbee_error_function(&self) -> ResponseCommand {
//         todo!()
//     }
//
//     fn hydrophone_error_function(&self) -> ResponseCommand {
//         todo!()
//     }
//
//     fn assign_device_response_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         todo!()
//     }
//
//     fn mac_address_response_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         todo!()
//     }
//
//     fn ping_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
//         todo!()
//     }
// }


struct ProtocolMQTT;


impl Protocol for ProtocolMQTT{

    fn log_recovery_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand{
        let command = data[0];
        let mut real_data: Vec<&str> = data[3..].to_vec();

        match command {
            "23" => self.read_response_function(device, &mut real_data, conn),
            "30" => self.feed_log_function(device, &real_data, conn),
            "31" => self.read_response_vars_other_function(device, &real_data, conn),
            "32" => self.write_response_vars_other_function(device, &real_data, conn),
            "33" => self.feed_log_confirmation_function(device, &real_data, conn),
            "37" => self.log_accumulation_day_function(device, &real_data, conn),
            "40" => self.rssi_function(device, &real_data, conn),
            "44" => self.log_sound_function_x2(device, &real_data, conn),
            "50" => self.log_sensor_do_temp_function(device, &real_data, conn),
            "52" => self.log_alarms_function(device, &real_data, conn),
            "80" => self.log_status_device_function(device, &real_data, conn),
            "81" => self.log_status_uc_function(device, &real_data, conn),
            "82" => self.log_sound_function(device, &real_data, conn),
            "83" => self.log_dosage_hydro_function(device, &real_data, conn),
            "88" => self.assign_device_response_function(device, &real_data, conn),
            "4E" => self.hydrophone_error_function(),
            "4F" => self.xbee_error_function(),

            _ => self.ack_response_function()
        };
        (None, MessageMode::LogRecovery, "".to_string(), None)
    }

    fn rssi_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn read_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn write_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_kg_hopper(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    // fn rssi_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
    //     // Es igual que protocolo UC
    //     ProtocolUC{}.rssi_function(device, data, conn)
    // }
    //
    // fn read_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
    //     // Es igual que protocolo UC
    //     ProtocolUC{}.read_response_vars_other_function(device, data, conn)
    // }
    //
    // fn write_response_vars_other_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
    //     // Es igual que protocolo UC
    //     ProtocolUC{}.write_response_vars_other_function(device, data, conn)
    // }

    // fn log_kg_hopper(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
    //     let mut placeholder: HashMap<String, String> = HashMap::new();
    //     let hex_date = data[0..6].to_vec();
    //     let real_date: Vec<String> = hex_date
    //         .iter()
    //         .map(|x| str_to_int(*x, 16).to_string())
    //         .collect();
    //     let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
    //     let device_position = str_to_int(data[6], 16);
    //     let devices = device.get_children_devices(conn).unwrap();
    //     let target_device_id = &devices[device_position as usize];
    //
    //     let value = str_to_int(data[7..=10].to_vec().join("").as_str(), 16) as f32;
    //     let var_id = target_device_id.get_variable("0E9F", conn);
    //
    //     if let Some(var) = var_id {
    //         placeholder.insert(var.name, value.to_string());
    //
    //         target_device_id.insert_into_logs(var.base_var_id, &timestamp, value, device.cycle_id, "0E9F",None, conn);
    //     }
    //
    //     let value_gr_dosage = str_to_int(data[11..=12].to_vec().join("").as_str(), 16) as f32;
    //     let var_id = target_device_id.get_variable("0CEB", conn);
    //
    //     if let Some(var) = var_id {
    //         placeholder.insert(var.name, value_gr_dosage.to_string());
    //
    //         target_device_id.insert_into_logs(var.base_var_id, &timestamp, value_gr_dosage, device.cycle_id, "0CEB",None, conn);
    //     }
    //
    //     let value_feed_rate = str_to_int(data[13..=14].to_vec().join("").as_str(), 16) as f32;
    //     placeholder.insert("Tasa de alimentacion".to_string(), value_feed_rate.to_string());
    //
    //     (None, MessageMode::ModeLogResponse, "Log de capacitancia".to_string(), Some(placeholder))
    // }

    fn log_alarms_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        let device_position = str_to_int(data[6], 16);
        let alarms = str_to_int(data[7..11].join("").as_str(), 16);
        let devices = device.get_children_devices(conn).unwrap();
        let target_device_id = &devices[device_position as usize];
        let capacitancia = str_to_int(data[11], 16);
        target_device_id.insert_alarm(alarms, conn);
        let placeholder: HashMap<String, String> = HashMap::from([
            ("Capacitancia".to_string(), capacitancia.to_string()),
            ("Dispositivo".to_string(), target_device_id.name.clone())
        ]);
        (None, MessageMode::ModeLogResponse, "Log de alarma".to_string(), Some(placeholder))
    }

    fn log_accumulation_day_function(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    fn log_sensor_do_temp_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    // fn log_sensor_do_temp_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
    //     ProtocolUC{}.log_sensor_do_temp_function(device, data, conn)
    // }

    /// Parse a feed log from UC 2.0
    fn feed_log_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        let mut placeholder: HashMap<String, String> = HashMap::new();
        let hex_date = data[0..6].to_vec();
        let real_date: Vec<String> = hex_date
            .iter()
            .map(|x| str_to_int(*x, 16).to_string())
            .collect();
        let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
        let value = str_to_int(data[6..10].to_vec().join("").as_str(), 16) as f32;
        let var_id = device.get_variable("0D49", conn);

        // if let Some(var) = var_id {
        //     placeholder.insert(var.name, value.to_string());
        //
        //     device.insert_into_logs(var.base_var_id, &timestamp, value,device.cycle_id, "0D49",None, conn);
        // }

        let empty_feeders = str_to_int(data[10..12].to_vec().join("").as_str(), 16);
        let children = device.get_feeders(conn).unwrap();
        let mut devices_vec: Vec<i32> = Vec::new();
        // Iteramos sobre cada alimentador y creamos el estado de las tolvas
        for (i, child) in children.iter().enumerate(){
            if empty_feeders & 2i32.pow(i as u32) > 0{
                child.create_device_status(&timestamp, DeviceStatus::Empty, conn);
            }else{
                child.create_device_status(&timestamp, DeviceStatus::Full, conn);
            }
            devices_vec.push(child.id);
        }
        bulk_clean_alarms(&devices_vec, None, conn);
        (None, MessageMode::ModeLogResponse, "Log acumulado x hora".to_string(), Some(placeholder))
    }

    /// Parse a feed log confirmation
    fn feed_log_confirmation_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        let mut placeholder: HashMap<String, String> = HashMap::new();
        let hex_date = data[0..6].to_vec();
        let real_date: Vec<String> = hex_date
            .iter()
            .map(|x| str_to_int(*x, 16).to_string())
            .collect();
        let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
        let value = str_to_int(data[6..8].join("").as_str(), 16) as f32;
        let feeders = BinaryString::from_hex(data[8..10].join("")).unwrap();
        let mut total_feeders = feeders.to_string();
        let feeders = feeders.add_spaces().unwrap();
        let var_id = device.get_variable("0CEB", conn);
        total_feeders = total_feeders.replace("0", "");
        // if let Some(var) = var_id {
        //     placeholder.insert(var.name, value.to_string());
        //     placeholder.insert("Alimentadores".to_owned(), feeders.to_string());
        //
        //     device.insert_into_logs(var.base_var_id, &timestamp, value,device.cycle_id, "0CEB",Some(&total_feeders.len().to_string()), conn);
        // }

        // Iteramos sobre cada alimentador y creamos el estado de las tolvas
        let children = device.get_feeders(conn).unwrap();
        let comm_feeders = str_to_int(data[8..10].join("").as_str(), 16);
        let mut devices_vec: Vec<i32> = Vec::new();
        for (i, child) in children.iter().enumerate(){
            if comm_feeders & 2i32.pow(i as u32) > 0{
                devices_vec.push(child.id);
            }
        }

        bulk_update_communication(&devices_vec, conn);
        (None, MessageMode::ModeLogResponse, "Log de confirmacion alimentacion".to_string(), Some(placeholder))
    }

    /// Parse a log sound
    fn log_sound_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        let raw_data = &data[6..data.len()];
        let date: Vec<&str> = data[..6].to_vec();
        let var = device.get_variable("DDE0", conn).unwrap();
        let now = chrono::Utc::now();
        let now = now.naive_utc();

        let start = now - chrono::Duration::hours(5i64);
        let start_date = start - chrono::Duration::hours(start.hour() as i64) + chrono::Duration::hours(5i64) - chrono::Duration::minutes(start.minute() as i64) - chrono::Duration::seconds(start.second() as i64);
        if device.mode == Some("hydro".to_string()) {
            let hydrophone_analysis = device.get_or_create_hydro_now(start_date, now, conn);
            match hydrophone_analysis {
                Ok(hydro) => {
                    let sound_data = &data[9..];
                    let sound_a = sound_data[2];
                    let sound_b = sound_data[sound_data.len() - 3];
                    hydro.create_lines(sound_a, sound_b, now, conn);
                },
                Err(r) => info!("Error al crear hydrophone line {}", r)
            }
        }
        // Obtenemos la variable indicador de duracion de sonido
        let real_date: Vec<String> = date
            .iter()
            .map(|x| str_to_int(*x, 16).to_string())
            .collect();
        let mut timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
        // Iteramos sobre la data y obtenemos cada valor de sonido
        // for value in raw_data.iter(){
        //     // Sacamos el valor como f32
        //     debug!("Insertando sonido {}...", value);
        //     let value = (*value).parse::<i32>().unwrap() as f32;
        //     device.insert_into_logs(var.base_var_id, &timestamp, value, device.cycle_id, "DDE0", None, conn);
        //
        //     timestamp += chrono::Duration::seconds(10i64);
        // };
        (None, MessageMode::ModeLogResponse, "Log de sonido".to_string(), None)
    }

    fn log_sound_function_x2(&self, _device: &Device, _data: &Vec<&str>, _conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        todo!()
    }

    /// Parse a log dosago from an hydrophone
    fn log_dosage_hydro_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        let value = str_to_int(data[7], 10) as f32;

        // Agregamos la descripcion de acuerdo a la trama
        let description = match data[6]{
            "P" => "cebo power on",
            "S" => "cebo start cycle",
            "-P" => "-cebo power on",
            "-S" => "-cebo start cycle",
            "-N" => "-",
            _ => ""
        };

        // Obtenemos la variable de dosificacion por hidrofono
        let var = device.get_variable("DDE2", conn).unwrap();

        let date: Vec<&str> = data[..6].to_vec();
        let placeholder: HashMap<String, String> = HashMap::from([
            (var.name.clone(), value.to_string()),
            ("descripcion".to_string(), description.to_string()),
        ]);

        let real_date: Vec<String> = date
            .iter()
            .map(|x| str_to_int(*x, 16).to_string())
            .collect();
        let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
        // device.insert_into_logs(var.base_var_id, &timestamp, value, device.cycle_id, "DDE2", Some(description), conn);
        (None, MessageMode::ModeLogResponse, "Log dosis x sonido".to_string(), Some(placeholder))
    }

    /// Parse a log status
    fn log_status_uc_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        let date: Vec<&str> = data[..6].to_vec();
        let real_date: Vec<String> = date
            .iter()
            .map(|x| str_to_int(*x, 16).to_string())
            .collect();
        let temperature = str_to_int(data[7], 16);
        let signal = str_to_int(data[6], 10);
        let placeholder: HashMap<String, String> = HashMap::from([
            ("UC".to_string(), device.name.clone()),
            ("Senal".to_string(), signal.to_string()),
            ("Temp".to_string(), temperature.to_string())
        ]);
        let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();

        device.insert_status_log(&timestamp, Some(temperature), signal, None, None, conn);
        (None, MessageMode::ModeLogResponse, "Log de Status UC".to_string(), Some(placeholder))
    }

    // Parse a log status device
    fn log_status_device_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        let date: Vec<&str> = data[..6].to_vec();

        // Transformamos todos los datos de &str a string de un entero en decimal
        let real_date: Vec<String> = date
            .iter()
            .map(|x| str_to_int(*x, 16).to_string())
            .collect();

        let timestamp = chrono::NaiveDateTime::parse_from_str(real_date.join(" ").as_str(), "%H %M %S %d %m %y").unwrap();
        let devices = device.get_children_devices(conn);
        let device_position = str_to_int(data[6], 16) as usize;
        let signal = str_to_int(data[7], 16);
        let mut battery: f32 = 0.0;
        let mut panel: f32 = 0.0;
        let mut placeholder: HashMap<String, String> = HashMap::new();
        if let Some(children) = devices{
            // Si no tiene alimentadores
            if children.is_empty(){
                return (None, MessageMode::ModeLogResponse, "Log de Status UC".to_string(), None);
            }
            let child = &children[device_position];

            let battery_var = child.get_variable("0D8D", conn);
            if let Some(bv) = battery_var{
                battery = bv.decode(data[8..12].join(" ").as_str()).parse::<f32>().unwrap();
            }

            let panel_var = child.get_variable("0D86", conn);
            if let Some(bv) = panel_var{
                panel = bv.decode(data[12..=15].join(" ").as_str()).parse::<f32>().unwrap();
            }

            placeholder.insert("Dispositivo".to_string(), child.name.clone());
            placeholder.insert("Senal".to_string(), signal.to_string());
            placeholder.insert("Bateria".to_string(), battery.to_string());
            placeholder.insert("Panel".to_string(), panel.to_string());

            let var_id = child.get_variable("4442", conn);

            if let Some(var) = var_id {
                // Actualizamos la variable RSSI
                var.update(&signal.to_string(), conn);
            }
            child.update_rssi_timestamp(conn, &timestamp);
            child.insert_status_log(&timestamp, None, signal, Some(battery), Some(panel), conn);
        }

        (None, MessageMode::ModeLogResponse, "Log de Status Dispositivo".to_string(), Some(placeholder))
    }

    fn xbee_error_function(&self) -> ResponseCommand {
        (None, MessageMode::XbeeError, "Problemas con el XBee".to_string(), None)
    }

    fn hydrophone_error_function(&self) -> ResponseCommand {
        (None, MessageMode::HydrophoneError, "No se pudo habilitar el hidrofono".to_string(), None)
    }

    /// Parse an assign response frame from UC
    fn assign_device_response_function(&self, device: &Device, data: &Vec<&str>, conn:  &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        if data.is_empty(){
            return (None, MessageMode::ModeReadResponse, "Respuesta de asignacion (OK)".to_string(), None);
        }
        let mac_addresses: String = data.join("");
        let children = device.get_children_devices(conn);
        let mut devices_not_assig: Vec<&str> = Vec::new();
        let mut placeholder: HashMap<String, String> = HashMap::new();

        // Si existe alimentadores
        if let Some(devices) = children{
            if !devices.is_empty(){
                for d in &devices{
                    // Si de los alimentadores asignados la uc no los asigno, desasignarlos
                    if  mac_addresses.contains(&d.address){
                        devices_not_assig.push(&d.name);
                        d.update_parent_device(conn);
                    }
                }
            };
            placeholder.insert("No asignados".to_string(), devices_not_assig.join(","));
        };
        (None, MessageMode::ModeReadResponse, "Respuesta de asignacion".to_string(), Some(placeholder))
    }

    fn mac_address_response_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        device.update_mac_address(data[0], conn);
        (None, MessageMode::ModeReadResponse, "Respuesta de MAC".to_string(), None)
    }

    fn ping_function(&self, device: &Device, data: &Vec<&str>, conn: &mut PooledConnection<PostgresConnectionManager<NoTls>>) -> ResponseCommand {
        if let Some(status) = device.status.as_ref(){
            match status.as_str(){
                "waiting" => device.update_status("intermitente", conn),
                "lost" => device.update_status("intermitente", conn),
                "intermitente" => device.update_status("ok", conn),
                _ => ()
            };
        }
        (None, MessageMode::ModeReadResponse, "Respuesta de PING".to_string(), None)
    }
}


/// Process packet received from MQTT broker
pub fn process_packet(payload: &str, topic: String, pool: Pool<PostgresConnectionManager<NoTls>>){
    info!("Message arrived: Topic {} -> {}", topic, payload);
    let topic_sp: Vec<&str> = topic.split('/').collect();
    let address: String;
    let mut conn = pool.get().unwrap();
    let payload_sp: Vec<&str> = payload.split(' ').collect();
    if topic_sp[1] == "x2"{
        address = payload_sp[4..12].join("");
    }else if topic_sp[1] == "uc"{
        address = topic_sp[3].to_string();
    }else{
        process::exit(1);
    }

    // Obtenemos el dispositivo segun la mac address recibida
    let option_device = get_device(address, &mut conn).unwrap_or_else(|err|{
        info!("Error {}", err);
        None
    });

    if let Some(dd) = option_device{
        // Sacamos el protocolo
        let protocol_name = dd.protocol.as_str();
        if protocol_name == "protocol.mqtt" {
            ProtocolMQTT.process_packet(&dd, &payload_sp, &mut conn);
        }
        // }else if protocol_name == "protocol.uc"{
        //     let variable_payload: Vec<&str> = payload_sp[15..payload_sp.len() - 1].to_vec();
        //     ProtocolUC.process_packet(&dd, &variable_payload, &mut conn);
        // }else if protocol_name == "protocol.x2"{
        //     let variable_payload: Vec<&str> = payload_sp[15..payload_sp.len() - 1].to_vec();
        //     ProtocolX2.process_packet(&dd, &variable_payload, &mut conn);
        // }
        dd.update_communication(&mut conn);
    }
}
