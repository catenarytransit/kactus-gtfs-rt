async fn fetch_swiftly(swiftly_id:&String) -> () {
    let routes_url = format!("https://transitime-api.goswift.ly/api/v1/key/81YENWXv/agency/{}/command/routesDetails", swiftly_id);

    println!(route_url);


}

fn get_routes_url(swiftly_id: &String) -> String {
    format!("https://transitime-api.goswift.ly/api/v1/key/81YENWXv/{}/octa/command/routesDetails", swiftly_id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_routes_url_test() {
        let swiftly_id = "octa".to_string();
        let routes_url = get_routes_url(&swiftly_id);
        assert_eq!(routes_url, "https://transitime-api.goswift.ly/api/v1/key/81YENWXv/octa/command/routesDetails");
    }
}