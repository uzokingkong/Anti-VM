use std::fs::File;
use std::io::Write;

fn main() {
    let mut file = File::create("helloworld.txt").expect("Failed to create file");
    file.write_all(b"helloworld").expect("Failed to write to file");
    println!("helloworld.txt created successfully!");
}
