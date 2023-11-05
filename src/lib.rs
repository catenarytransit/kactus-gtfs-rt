#[macro_use]
extern crate serde_derive;

pub fn parse_protobuf_message(
    bytes: &[u8],
) -> Result<gtfs_rt::FeedMessage, Box<dyn std::error::Error>> {
    let x = prost::Message::decode(bytes);

    if x.is_ok() {
        return Ok(x.unwrap());
    } else {
        return Err(Box::new(x.unwrap_err()));
    }
}

pub mod insert {

    use prost::Message;
    use redis::{Commands, Connection, RedisResult};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn insert_gtfs_rt_bytes(
        con: &mut Connection,
        bytes: &Vec<u8>,
        onetrip: &String,
        category: &String,
    ) {
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let key: String = format!("gtfsrt|{}|{}", &onetrip, &category);
        let _: () = con
            .set(key.clone(), bytes)
            .unwrap();

        let msg: Vec<u8> = bytes.clone();
        match con.publish::<String, Vec<u8>, u8>(key, msg) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error publishing message: {:?}", err);
            }
        }

        inserttimes(con, &onetrip, &category, &now_millis);
    }

    fn inserttimes(con: &mut Connection, onetrip: &String, category: &String, now_millis: &String) {
        let _: () = con
            .set(
                format!("gtfsrttime|{}|{}", &onetrip, &category),
                &now_millis,
            )
            .unwrap();

        let _: () = con
            .set(format!("gtfsrtexists|{}", &onetrip), &now_millis)
            .unwrap();
    }

    pub fn insert_gtfs_rt(
        con: &mut Connection,
        data: &gtfs_rt::FeedMessage,
        onetrip: &String,
        category: &String,
    ) {
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let bytes: Vec<u8> = data.encode_to_vec();

        let _: () = con
            .set(format!("gtfsrt|{}|{}", &onetrip, &category), bytes.to_vec())
            .unwrap();

        inserttimes(con, &onetrip, &category, &now_millis);
    }
}
