use csv::ReaderBuilder;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn Error>> {
    // Open the CSV file
    let file = File::open("urls.csv")?;
    let reader = BufReader::new(file);

    // Create a CSV reader
    let mut csv_reader = ReaderBuilder::new().flexible(true).from_reader(reader);

    // Iterate over each record (line) in the CSV file
    for result in csv_reader.records() {
        // Unwrap the record
        let record = result?;

        // Check the number of fields in the record
        if record.len() > 0 {
            // Print the first field of the record
            println!("{}", record.get(0).unwrap());
        }
    }

    Ok(())
}
