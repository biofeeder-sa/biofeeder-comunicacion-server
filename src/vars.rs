use core::str;
use postgres::Client;
use log::{info, debug};

/// Convert string to i32
pub fn str_to_int(str_integer: &str, radix: u32) -> i32{
    i32::from_str_radix(str_integer, radix).unwrap_or(0)

}

/// Convert string to i32
pub fn str_to_int64(str_integer: &str, radix: u32) -> i64{
    let str_int64 = str_integer.replace(" ", "");
    i64::from_str_radix(str_int64.as_str(), radix).unwrap_or(0)

}

/// Convert string to u32
pub fn ustr_to_int(str_integer: &str, radix: u32) -> u32{
    u32::from_str_radix(str_integer, radix).unwrap_or(0)
}

/// Structure of a device's variable
pub struct Var{
    pub id: i32,
    pub size: i32,
    pub format: String,
    pub name: String,
    pub base_var_id: i32,
    pub value: String,
    pub code: Option<String>
}

impl Var{

    /// Create new Var
    pub fn new(id: i32, size: i32, format: String, name: String, base_var_id: i32, value: String, code: Option<String>) -> Self{
        Self{
            id,
            size,
            format,
            name,
            base_var_id,
            value,
            code
        }
    }

    /// Each variable has its value representation
    /// decode raw data from device and return the representation
    pub fn decode(&self, raw_data: &str) -> String{
        let data: Vec<&str> = raw_data.split(' ').collect();
        match self.format.as_str(){
            "longitude" => decode_longitude(data),
            "latitude" => decode_latitude(data),
            "bcd" => decode_bcd(data),
            "dec" => decode_dec(data),
            "b31" => decode_b31(data),
            "b22" => decode_b22(data),
            "b11" => decode_b11(data),
            "asc" => decode_asc(data),
            "hex" => decode_hex(data),
            "jf2" => decode_jf2(data),
            "jf4" => decode_jf4(data),
            "dec2b" => decode_dec2b(data),

            _ => "".to_string()
        }
    }

    pub fn update(&self, value: &str, conn: &mut Client){
        debug!("Actualizando variable {} con {}", self.id, value);
        let now = chrono::Utc::now();
        let now = now.naive_utc();
        let statement = conn.prepare("UPDATE var set value=$1, write_date=$3 where id=$2").unwrap();
        let result = conn.execute(&statement, &[&value, &self.id, &now]);
        match result{
            Ok(_response) => (),
            Err(e) => info!("Error ocurred {}", e)
        };
    }
}


/// Convert a collection of hexadecimal str to a collection of u32
fn human2u32(data: Vec<&str>) -> Vec<u32>{
    let hex_data: Vec<u32> = data
        .iter()
        .map(|d| u32::from_str_radix(d, 16).unwrap())
        .collect();
    hex_data
}

/// Decode decimal values
fn decode_dec(raw_data: Vec<&str>) -> String{
    let hex_value = raw_data.join("");
    let value = ustr_to_int(hex_value.as_str(), 16);
    value.to_string()
}

/// Decode longitude, data received from device
fn decode_longitude(raw_data: Vec<&str>) -> String{
    // Convertimos la informacion hexadecimal en un vector de bytes u32
    let hex_data: Vec<u32> = human2u32(raw_data);

    // Cada item del vector lo transformamos en su representacion de caracter
    let chr_value: Vec<String> = hex_data
        .iter()
        .map(|d| char::from_u32(*d).unwrap().to_string())
        .collect();

    // Se une y se transforma en bytes
    let value_gsm1 = chr_value[..3].join("").parse::<u32>().unwrap_or(0);
    let value_gsm2 = chr_value[3..5].join("").parse::<u32>().unwrap_or(0);
    let value_gsm3 = chr_value[5..9].join("").parse::<u32>().unwrap_or(0);
    let value_gsm4 = String::from(&chr_value[9]);
    let value_gsm = value_gsm1 + (value_gsm2 + value_gsm3 / 10000) / 60;

    // Si el ultimo caracter es W o S el valor es negativo
    let value =  match value_gsm4.as_str(){
        "W" => -(value_gsm as i32),
        "S" => -(value_gsm as i32),
        _ => value_gsm as i32
    };
    value.to_string()
}

fn decode_latitude(raw_data: Vec<&str>) -> String{
    // Convertimos la informacion hexadecimal en un vector de bytes u32
    let hex_data: Vec<u32> = human2u32(raw_data);
    // Cada item del vector lo transformamos en su representacion de caracter
    let chr_value: Vec<String> = hex_data
        .iter()
        .map(|d| char::from_u32(*d).unwrap().to_string())
        .collect();


    // Se une y se transforma en bytes
    let value_gsm1 = chr_value[..2].join("").parse::<u32>().unwrap_or(0);

    let value_gsm2 = chr_value[2..4].join("").parse::<u32>().unwrap_or(0);
    let value_gsm3 = chr_value[4..8].join("").parse::<u32>().unwrap_or(0);
    let value_gsm4 = String::from(&chr_value[8]);
    let value_gsm = value_gsm1 + (value_gsm2 + value_gsm3 / 10000) / 60;

    // Si el ultimo caracter es W o S el valor es negativo
    let value =  match value_gsm4.as_str(){
        "W" => -(value_gsm as i32),
        "S" => -(value_gsm as i32),
        _ => value_gsm as i32
    };
    value.to_string()
}

/// Decode raw data in decimal
fn decode_bcd(raw_data: Vec<&str>) -> String{
    // Retornamos lo mismo
    let result: Vec<String> = raw_data
        .iter()
        .map(|x| i32::from_str_radix(x, 16).unwrap().to_string())
        .collect();
    result.join(" ")
}

/// Decode a bcd with 3 bytes for integer and 1 byte for decimal
/// E.g: 00 00 25 25
/// Returns "25.25"
fn decode_b31(raw_data: Vec<&str>) -> String{
    let integer: String = raw_data[0..3].join("");
    let integer = str_to_int(integer.as_str(), 10);
    let decimal: String = raw_data[3..raw_data.len()].join("");
    let decimal = str_to_int(decimal.as_str(), 10);

    let result = integer as f64 + (decimal as f64 / 100.0);
    result.to_string()
}

/// Decode a bcd with 2 bytes for integer and 2 byte for decimal
/// E.g: 00 10 25 25
/// Returns "10.2525"
fn decode_b22(raw_data: Vec<&str>) -> String{
    let integer: String = raw_data[0..2].join("");
    let integer = str_to_int(integer.as_str(), 10);
    let decimal: String = raw_data[2..=3].join("");
    let decimal = str_to_int(decimal.as_str(), 10);

    let result = integer as f64 + (decimal as f64 / 10000.0);
    result.to_string()
}

/// Decode a bcd with 1 byte for integer and 1 byte for decimal
/// E.g: 10 25
/// Returns "10.25"
fn decode_b11(raw_data: Vec<&str>) -> String{
    let integer: &str = raw_data[0];
    let integer = str_to_int(integer, 10);
    let decimal: &str = raw_data[1];
    let decimal = str_to_int(decimal, 10);

    let result = integer as f64 + (decimal as f64 / 100.0);
    result.to_string()
}

/// Decode and hexadecimal to ascii
fn decode_asc(raw_data: Vec<&str>) -> String{
    let data = raw_data.join("");
    let result = hex::decode(data).unwrap();
    let str_result = str::from_utf8(result.as_slice());
    let result = str_result.unwrap().to_string();
    result.replace("\u{0}", "")
}

/// Decode and hexadecimal value
fn decode_hex(raw_data: Vec<&str>) -> String{
    raw_data.join(" ")
}

/// Decode hexadecimal to number
/// 1 byte for integer, 1 byte for decimal
fn decode_jf2(raw_data: Vec<&str>) -> String{
    let integer: &str = raw_data[0];
    let integer = str_to_int(integer, 16);
    let decimal: &str = raw_data[1];
    let decimal = str_to_int(decimal, 16);

    let result = integer as f64 + (decimal as f64 / 100.0);
    result.to_string()
}

/// Decode hexadecimal to number
/// 2 bytes for integer, 2 bytes for decimal
fn decode_jf4(raw_data: Vec<&str>) -> String{
    let integer: String = raw_data[0..2].join("");
    let integer = str_to_int(integer.as_str(), 16);
    let decimal: String = raw_data[2..raw_data.len()].join("");
    let decimal = str_to_int(decimal.as_str(), 16);

    let result = integer as f64 + (decimal as f64 / 100.0);
    result.to_string()
}

/// Decode an hexadecimal to a floating number
fn decode_dec2b(raw_data: Vec<&str>) -> String{
    let integer: String = raw_data.join("");
    let integer = str_to_int(integer.as_str(), 16);
    let value = integer as f32 / 100.0;
    value.to_string()
}