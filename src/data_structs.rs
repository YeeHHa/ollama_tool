use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Debug)]
pub struct Models {
    pub models: Vec<Model>,    
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Model {
    pub name: String,
    pub model: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: Detail 
}

impl Model {
    pub fn display(&self) {
        println!("Model Name: {}", self.name);
        println!("Model ID: {}", self.model);
        println!("Modified At: {}", self.modified_at);
        println!("Size: {} bytes", self.size);
        self.details.display();
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Detail {
    pub parent_model: String,
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub parameter_size: String,
    pub quantization_level: String
}

impl Detail {
    pub fn display(&self) {
        println!("Parent Model: {}", self.parent_model);
        println!("Format: {}", self.format);
        println!("Family: {}", self.family);

        if let Some(families) = &self.families {
            for fam in families {
                println!(" - {}", fam);
            }
        }

        println!("Parameter Size: {}", self.parameter_size);
        println!("Quantization Level: {}", self.quantization_level);
    }
}