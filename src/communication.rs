//Toda la comunicacion con biomatic esta aqui
use std::{process, sync::RwLock, thread, time::Duration};
use std::collections::HashMap;
use paho_mqtt::{AsyncClient, ConnectOptionsBuilder, CreateOptionsBuilder, Message, MQTT_VERSION_5};
use log::{info, error};
use r2d2_postgres::PostgresConnectionManager;
use r2d2::Pool;
use postgres::NoTls;
use crate::protocols::process_packet;
use crate::db;

const BROKER: &str = "xb4cdf95-internet-facing-a67c98d9ae4b13a0.elb.us-east-1.amazonaws.com";
const PASSWORD: &str = "BiofeederMQTT";
const USERNAME: &str = "biofeeder";
const TOPIC: &str = "server/uc/#";

type ConnectionPool = RwLock<Pool<PostgresConnectionManager<NoTls>>>;

pub struct MqttClient{
    db_name: String,
    cli: AsyncClient,
}

/// Function that consuming al arrived messages from MQTT broker
/// This is spawned for each database client and spawn a thread
/// for each arrived message
fn receive(cli: &AsyncClient, msg: Option<Message>){

    // Si existe algun mensaje lo procesamos
    if let Some(msg) = msg {
        let topic = msg.topic();
        let data = cli.user_data().unwrap();
        let mut pool_clone: Option<Pool<PostgresConnectionManager<NoTls>>> = None;
        if let Some(lock) = data.downcast_ref::<ConnectionPool>() {

            // Obtenemos el pool de conexiones
            let connection_pool = lock.read().unwrap();
            pool_clone = Some(connection_pool.clone());
            // if let Some(connection_pool) = connection_pool{
            //     pool_clone = Some(connection_pool.clone());
            // }
        }

        let payload_str = msg.payload_str();
        let payload = payload_str.to_string();
        let topic = topic.to_owned();
        if let Some(pool_clone) = pool_clone{
            // Generamos hilo por cada mensaje recibido
            thread::spawn(move || process_packet(payload.as_str(), topic, pool_clone));
        }
    }
}

fn on_connect_failure(cli: &AsyncClient, _msgid: u16, rc: i32) {
    error!("Connection attempt failed with error code {}.\n", rc);
    thread::sleep(Duration::from_millis(2500));
    cli.reconnect_with_callbacks(on_connect_success, on_connect_failure);
}

fn on_connect_success(cli: &AsyncClient, _msgid: u16){
    info!("Connection succeeded");
    let data = cli.user_data().unwrap();
    info!("Subscribing to topic: {}", TOPIC);
    let new_topics: Vec<String> = vec!["server/uc/#".to_string(), "server/x2/#".to_string()];
    let qos = vec![0; new_topics.len()];
    // Subscribimos al topico
    cli.subscribe_many(&new_topics, &qos);
    // cli.subscribe(TOPIC, 0);

}

impl MqttClient{

    /// Create a new MQTT Client with ID as database name
    pub fn new(db_name: String) -> Self{
        let db_name = db_name.as_str();
        let pool = db::connect(db_name).unwrap();
        let mut topics: Vec<String> = Vec::new();
        let mut conn = pool.get().unwrap();

        // Creamos las opciones para el client MQTT
        let opts = CreateOptionsBuilder::new()
            .server_uri(BROKER)
            .client_id("server_biomatic_aws".to_owned() + db_name)
            .user_data(Box::new(RwLock::new(pool)))
            .finalize();

        // Creamos cliente mqtt
        let cli = AsyncClient::new(opts).unwrap_or_else(|err| {
            info!("Error al crear el cliente: {:?}", err);
            process::exit(1);
        });

        // Cerramos la conexion con la base de datos, no lo necesitamos mas
        // conn.close();

        Self{
            db_name: db_name.to_string(),
            cli,
        }
    }

    /// Connect to broker MQTT, if the connection is not posible
    /// program ends
    pub fn connect(&mut self){

        // Callback para reconectarse con el broker
        self.cli.set_connection_lost_callback(|cli: &AsyncClient| {
            info!("Connection lost. Attempting reconnect.");
            thread::sleep(Duration::from_millis(2500));
            cli.reconnect_with_callbacks(on_connect_success, on_connect_failure);
        });
        // Agregamos callback para recepcion de mensajes
        self.cli.set_message_callback(receive);

        // Opciones para conectar al broker MQTT
        let conn_options = ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(20))
            .mqtt_version(MQTT_VERSION_5)
            .clean_session(true)
            .password(PASSWORD)
            .user_name(USERNAME)
            .finalize();

        // Connexion con el broker
        self.cli.connect_with_callbacks(conn_options, on_connect_success, on_connect_failure);

        info!("Mqtt client {} subscrito..", self.db_name);

        // Sin esto el programa termina
        // loop {
        //     thread::sleep(Duration::from_millis(1000));
        // }
    }
}
