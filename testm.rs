use csv::ReaderBuilder;
use serde::{Serialize, Deserialize};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Serialize, Deserialize)]
struct AvailableAgencies {
    agencies: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Open the CSV file
    let file = File::open("urls.csv")?;
    let reader = BufReader::new(file);

    // Create a CSV reader
    let mut csv_reader = ReaderBuilder::new().flexible(true).from_reader(reader);

    // Create a vector to hold the available agencies
    let mut available_agencies = AvailableAgencies {
        agencies: Vec::new(),
    };

    // Iterate over each record (line) in the CSV file
    for result in csv_reader.records() {
        // Unwrap the record
        let record = result?;

        // Check the number of fields in the record
        if record.len() > 0 {
            // Get the first field of the record
            let agency = record.get(0).unwrap().to_owned();

            // Store the agency in the vector
            available_agencies.agencies.push(agency);
        }
    }

    // Serialize the available agencies to JSON
    let json_data = serde_json::to_string(&available_agencies)?;

    // Write the JSON data to a file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("available_agencies.json")?;

    file.write_all(json_data.as_bytes())?;

    Ok(())
}
