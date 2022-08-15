extern crate core;

mod communication;
mod protocols;
mod db;
mod device;
mod vars;
mod message;

use log::{info, LevelFilter};
use communication::MqttClient;
use configparser::ini::Ini;
use std::{process, thread};
use std::borrow::BorrowMut;
use std::time::Duration;

fn main() {

    simple_logging::log_to_file("/var/log/communication.log", LevelFilter::Info);
    // Leemos el archivo db.ini para obtener la base de los clientes
    let mut config = Ini::new();
    let lector = config.load("/var/local/db-config.ini").unwrap_or_else(|err|{
        info!("Error al leer el archivo {}", err);
        process::exit(1);
    });

    let mut mqtt_client: Option<MqttClient> = None;
    if let Some(databases) = lector.get("database"){
        if let Some(db_name) = databases.get("db_name").unwrap(){
            info!("Creating and connecting mqtt client for {}", db_name);
            mqtt_client = Some(MqttClient::new(db_name.to_owned()));

            // Conectamos el cliente MQTT e iniciamos el proceso
            // thread::spawn(move || mqtt_client.connect());
        }
    }
    if let Some(mqtt) = mqtt_client.borrow_mut(){
        mqtt.connect();
    }
    // Eliminanos referencia, ya no es necesaria
    drop(lector);

    loop{
        // Loop infinito para que no acabe el thread principal
        thread::sleep(Duration::from_millis(1000));
    }
}
