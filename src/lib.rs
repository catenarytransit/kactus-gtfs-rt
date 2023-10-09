#[macro_use]
extern crate serde_derive;

pub fn parse_protobuf_message(bytes: &[u8]) -> Result<gtfs_rt::FeedMessage, Box<dyn std::error::Error>> {
    let x = prost::Message::decode(bytes);

    if x.is_ok() {
        return Ok(x.unwrap());
    } else {
        return Err(Box::new(x.unwrap_err()));
    }
}

pub mod insert {

    use prost::Message;
    use protobuf::{CodedInputStream, Message as ProtobufMessage};
    use redis::Commands;
    use redis::RedisError;
    use redis::{Client as RedisClient, Connection, RedisResult};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
}
