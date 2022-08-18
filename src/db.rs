use std::process;
use configparser::ini::Ini;
use log::{error, info};
use postgres::config::SslMode;
use native_tls::{Certificate, TlsConnector};
use r2d2_postgres::PostgresConnectionManager;
use r2d2::Pool;
use std::fs;
use chrono;
use postgres::NoTls;

/// Connect to database an return an optional connection
/// If errors occurs exit program
fn get_conn(dbname: &str, host: &str, db_user: &str, password: &str, db_port: &str) -> Option<Pool<PostgresConnectionManager<NoTls>>>{

    // Tenemos los parametros para conexion a la base de datos
    let params =format!("host={} user={} dbname={} password={} port={}", host, db_user, dbname, password, db_port);
    let manager = PostgresConnectionManager::new(
        params.as_str().parse().unwrap(),
        NoTls,
    );

    let pool = Pool::builder()
        .max_size(200)
        .build(manager);

    match pool{
        Ok(connection) => Some(connection),
        Err(error) => {
            info!("Ocurrio un error al crear conexion {}", error);
            None
        }
    }
}


/// Connect to database
pub fn connect(dbname: &str) -> Option<Pool<PostgresConnectionManager<NoTls>>>{
    // Leemos db.ini para obtener las credenciales de las bases de datos
    let mut config = Ini::new();
    let parser = config.load("/var/local/db-config.ini").unwrap_or_else(|err|{
        info!("No se pudo leer el archivo {}", err);
        process::exit(1);
    });

    // Declaramos variables
    let host: &String;
    let db_user: &String;
    let db_password: &String;
    let mut db_port: &String = &String::from("5432");


    let section = parser.get("database").unwrap();

    // Obtenemos las credenciales
    if let Some(i) = section.get("host").unwrap(){
        host = i;
    }else{
        return None
    }
    if let Some(i) = section.get("db_user").unwrap(){
        db_user = i;
    }else{
        return None
    }
    if let Some(i) = section.get("db_password").unwrap(){
        db_password = i;
    }else{
        return None
    }
    if let Some(i) = section.get("db_port").unwrap(){
        db_port = i;
    }

    // Conectamos a la base de datos
    let connection = get_conn(dbname, host.as_str(), db_user.as_str(), db_password.as_str(), db_port.as_str());
    connection
}