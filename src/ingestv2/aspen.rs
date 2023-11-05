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
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let _con = redisclient.get_connection().unwrap();

    let generating_vehicles = [
        "f-mta~nyc~rt~mnr",
        "f-mta~nyc~rt~lirr",
        "f-roamtransit~rt",
        "f-bart~rt",
    ];

    let vehicles_exist = generating_vehicles.contains(&agency) || vehicles_exist;
}
