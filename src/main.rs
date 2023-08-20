use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value as JsonValue;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::{stdin, stdout, Write};

#[derive(Debug, Deserialize, Serialize)]
struct Skin {
    #[serde(rename = "Item Shortname")]
    item_shortname: String,
    #[serde(rename = "Skins")]
    skins: Vec<u32>,
    api_name: String,
}

async fn read_json_file(file_path: &str) -> Result<Vec<Skin>, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let data: serde_json::Value = serde_json::from_str(&contents)?;
    let mut skins: Vec<Skin> = serde_json::from_value(data["Skins"].clone())?;

    for skin in &mut skins {
        if skin.api_name == "" {
            let url = format!(
                "https://wiki.facepunch.com/rust/item/{}",
                skin.item_shortname
            );
            match get_value_from_webpage(&url).await {
                Ok(value) => {
                    println!("Value from webpage: {}", value);
                    skin.api_name = value;
                }
                Err(err) => {
                    println!("Probalby a dlc item: {}", err)
                }
            }
        }

        if skin.api_name == "" {
            println!("No value found for {}", skin.item_shortname);
            continue;
        }

        let url = format!(
            "https://rust.scmm.app/api/item?type={}&count=5000",
            skin.api_name
        );

        let response = reqwest::get(&url).await?;

        if response.status().is_success() {
            let body = response.text().await?;
            let json_data: JsonValue = serde_json::from_str(&body)?;

            if let Some(items) = json_data["items"].as_array() {
                println!("Found {} skin ids for item  {}", items.len(), skin.api_name);
                for item in items {
                    if let Some(action) = item["actions"][1].as_object() {
                        let url_value = action["url"]
                            .as_str()
                            .and_then(|url| {
                                url.strip_prefix(
                                    "https://steamcommunity.com/sharedfiles/filedetails/?id=",
                                )
                            })
                            .map(|id| id.to_owned())
                            .unwrap_or_default();
                        if url_value != "" {
                            println!("Url: {}", action["url"]);
                            println!("URL Value: {}", url_value);
                            skin.skins.push(url_value.parse::<u32>().unwrap());
                        }
                    }
                }
            }
        }
    }

    let updated_data = json!({ "Skins": skins });
    let updated_json = serde_json::to_string_pretty(&updated_data)?;

    let new_file_path = "files/Skins.json";
    fs::write(new_file_path, updated_json)?;

    Ok(skins)
}

async fn check_internet_connection() -> bool {
    match reqwest::get("https://rust.scmm.app").await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

async fn get_value_from_webpage(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let document = Html::parse_document(&body);

        let selector = Selector::parse("div.pagetitle > a:nth-child(2)").unwrap();

        let value = match document.select(&selector).next() {
            Some(element) => element.inner_html(),
            None => return Err("Unable to find the second <a> tag".into()),
        };

        Ok(value)
    } else {
        Err("Request failed".into())
    }
}

fn pause() {
    let mut stdout = stdout();
    stdout.write(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read(&mut [0]).unwrap();
}

#[tokio::main]
async fn main() {
    println!("Rust Skin ID Finder v1 by @drvlopes");
    println!("Checking internet connection...");
    if check_internet_connection().await {
        println!("Website is available. Continuing...");
    } else {
        println!("Website is not available. Exiting...");
        return;
    }
    let file_path = "files/Skins_base_file.json";
    match read_json_file(file_path).await {
        Ok(skins) => {
            for skin in skins {
                println!("Item Shortname: {}", skin.item_shortname);
                for skin_id in skin.skins {
                    println!("Skin ID: {}", skin_id);
                }
                println!("API Name: {}", skin.api_name);
                println!("-----------------------");
            }
        }
        Err(err) => {
            eprintln!("Error reading JSON file: {}", err);
        }
    }
    pause();
}
