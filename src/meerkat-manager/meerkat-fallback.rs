use reqwest::Client as ReqwestClient;
use reqwest::StatusCode;
use std::time::Duration;
#[tokio::main]
async fn main() {
    let client = ReqwestClient::new();

    loop {
        let kactusserver = systemctl::Unit::from_systemctl("kactusserver.service").unwrap();
        let kactusingest = systemctl::Unit::from_systemctl("kactusingest.service").unwrap();

        let zot = systemctl::Unit::from_systemctl("zotgtfsrt.service").unwrap();

        let la = systemctl::Unit::from_systemctl("kactuslacmta.service").unwrap();

        let request = client.get("https://kactus.catenarymaps.org/").send().await;

        let pve_is_up = match request {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    println!("Meerkat is up");
                    true
                }
                _ => {
                    println!("Meerkat is down");
                    false
                }
            },
            Err(error) => {
                println!("Meerkat couldn't connect: {:?}", error);
                false
            }
        };

        if pve_is_up {
            // stop server
            let _ = kactusserver.stop();
            let _ = kactusingest.stop();
            let _ = zot.stop();
            let _ = la.stop();
        } else {
            // start server
            let _ = kactusserver.start();
            let _ = kactusingest.start();
            let _ = zot.start();
            let _ = la.start();
        }

        //sleep 1 sec

        let sleep_duration = Duration::from_millis(1000);
        println!("sleeping for {:?}", sleep_duration);
        std::thread::sleep(sleep_duration);
    }
}
