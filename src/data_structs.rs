use serde::{
    Deserialize, 
    Serialize
};
use std::fmt;



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

#[derive(Serialize, Deserialize, Debug)]
pub struct Chat {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<Tool>>,
    pub think: Option<bool>,
    pub stream: bool,
}

impl fmt::Display for Chat {

    fn fmt(&self, f :&mut fmt::Formatter) -> fmt::Result {

        write!(f, "Model: : {}\n", self.model)?;
        
        self.messages
            .iter()
            .for_each(|message| {
                let _ = write!(f, "-------------------------------\n");
                let _ = write!(f, "{}\n", message.content);
                let _ = write!(f, "-------------------------------\n");
            }
        );

        write!(f, "Tools: {:?}\n", self.tools)?;
        write!(f, "Stream: {:?}\n", self.stream)?;
        write!(f, "think: {:?}\n", self.think)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub thinking: Option<String>,
    pub images: Option<Vec<String>>,
    pub tool_calls: Option<Vec<Tool>>,

}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tool {
    pub name: Function 
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Function {
    pub name: String,
    pub description: String,
    pub arguments: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatResponse {
    pub model: String,
    pub created_at: String,
    pub message: Message,
    pub done_reason: Option<String>,
    pub done: bool,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u64>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
    pub logprobs: Option<LogProbs>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct LogProbs {
    pub tokens: Option<String>,
    pub logprobs: Option<Vec<f64>>,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Option<Vec<TopLogProb>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TopLogProb {
    pub token: String,
    pub logprob: f64,
    pub bytes: Vec<u8>,
}

