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
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn insert_gtfs_rt_bytes(
        con: &mut Connection,
        bytes: &Vec<u8>,
        onetrip: &str,
        category: &str,
    ) {
        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let key: String = format!("gtfsrt|{}|{}", &onetrip, &category);
        let _: () = con.set(key.clone(), bytes).unwrap();

        let msg: Vec<u8> = bytes.clone();
        let _xadd_result: RedisResult<String> = con.xadd(
            format!("{}-{}", &onetrip, &category),
            "*",
            &[(key.clone(), msg.clone())],
        );
        inserttimes(con, &onetrip, &category, &now_millis);
        let _ = con.set_read_timeout(Some(Duration::new(10, 0)));
    }
    pub fn insert_gtfs_rt(
        con: &mut Connection,
        data: &gtfs_rt::FeedMessage,
        onetrip: &str,
        category: &str,
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

    fn inserttimes(con: &mut Connection, onetrip: &str, category: &str, now_millis: &String) {
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

pub mod aspen {
    pub async fn send_to_aspen(
        agency: &str,
        vehicles_result: &Option<Vec<u8>>,
        trips_result: &Option<Vec<u8>>,
        alerts_result: &Option<Vec<u8>>,
        vehicles_exist: bool,
        trips_exist: bool,
        alerts_exist: bool,
        useexistingdata: bool,
    ) {
        //send data to aspen over tarpc
      //  let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        //let _con = redisclient.get_connection().unwrap();
    
        let generating_vehicles = [
            "f-mta~nyc~rt~mnr",
            "f-mta~nyc~rt~lirr",
            "f-roamtransit~rt",
            "f-bart~rt",
        ];
    
        let vehicles_exist = generating_vehicles.contains(&agency) || vehicles_exist;
    }
    
    
}