use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::PermissionsExt; // Unix-specific permissions

// Define a struct to represent a script
// Debug trait allows us to print it with {:?}
#[derive(Debug)]
struct Script {
    path: String,
    name: String,
    description: Option<String>,
}

// Function to extract description from a script file
// Takes a path as &str (borrowed string slice)
// returns Result<Option<String>, io::Error>
//  - Ok(Some(description)) if we found a description
//  - Ok(None) if no description
//  - Err(error) if we couldn't read the file
fn extract_description(path: &str) -> Result<Option<String>, io::Error> {
    // Open the file
    let file = fs::File::open(path)?;

    // wrap it in a buffered reader for efficient line-by-line reading
    let reader = io::BufReader::new(file);

    // iterate through lines
    for line_result in reader.lines() {
        let line = line_result?;
        
        // trim whitespace
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }
        // Check for different comment formats
        if let Some(desc) = trimmed.strip_prefix('#') {
            // Found # comment
            return Ok(Some(desc.trim().to_string()));
        } else if let Some(desc) = trimmed.strip_prefix("//") {
            return Ok(Some(desc.trim().to_string()));
        } else if let Some(desc) = trimmed.strip_prefix("--") {
            return Ok(Some(desc.trim().to_string()));
        } else {
            break;
        }
    }

    Ok(None)
}



fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <directory>", args[0]);
        return Ok(());
    }

    let directory = &args[1];
    let entries = fs::read_dir(directory)?;

    // now we collect Script structs instead of just strings
    let mut scripts: Vec<Script> = Vec::new();

    for entry_result in entries {
        let entry = entry_result?;
        let path = entry.path();
        
        if path.is_dir() {
            continue;
        }

        let metadata = fs::metadata(&path)?;
        let permissions = metadata.permissions();

        if permissions.mode() & 0o111 != 0 {
            // Extract just the filename
            let name = path
                .file_name() // gets filename
                .and_then(|n| n.to_str()) // converts &str if valid UTF-8
                .unwrap_or("unknown") // fallback if conversion fails
                .to_string();

            let path_str = path.to_str().unwrap_or("").to_string();

            // Extract description fro mhte script
            // We use unwrap_or(None) to convert Err to None
            // (If we can't read the file, just skip the description)
            let description = extract_description(&path_str)?;

            let script = Script {
                path: path.to_str().unwrap_or("").to_string(),
                name,
                description,
            };

            scripts.push(script);
        
            
        }
    }

    // print the results
    println!("found {} executable scripts:\n", scripts.len());
    for script in scripts {
        println!{"{}", script.name};
        if let Some(desc) = &script.description {
            println!("  {}", desc);
        } else {
            println!("  (no description)");
        }
        println!("  Path: {}", script.path);
        println!();
    }
    
    Ok(())
}
