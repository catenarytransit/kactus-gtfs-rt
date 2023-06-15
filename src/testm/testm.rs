use csv::ReaderBuilder;
use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;

#[derive(Serialize)]
struct AvailableAgencies {
    agencies: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Open the CSV file
    let file = File::open("urls.csv")?;
    let reader = BufReader::new(file);

    // Create a CSV reader
    let mut csv_reader = ReaderBuilder::new().flexible(true).from_reader(reader);

    // Create a vector to store the agency names
    let mut agencies: Vec<String> = Vec::new();

    // Iterate over each record (line) in the CSV file
    for result in csv_reader.records() {
        // Unwrap the record
        let record = result?;

        // Check the number of fields in the record
        if record.len() > 0 {
            // Store the first field of the record
            agencies.push(record.get(0).unwrap().to_string());
        }
    }

    // Create an instance of AvailableAgencies
    let available_agencies = AvailableAgencies { agencies };

    // Serialize the AvailableAgencies object to JSON
    let json_data = serde_json::to_string(&available_agencies)?;

    // Write the JSON data to a file
    let mut output_file = File::create("agencies.json")?;
    output_file.write_all(json_data.as_bytes())?;

    Ok(())
}
