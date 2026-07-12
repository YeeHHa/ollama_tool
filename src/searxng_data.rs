use serde::{
    Deserialize,
    Serialize
     
};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearxngParams {
    pub q: String,
    pub format: String,

}

impl SearxngParams {
    pub fn new(query: &String) -> SearxngParams{
        SearxngParams { 
            q: query.to_string(),
            format: "json".to_string(), 
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct SearxngResponse {

    pub query: String,
    pub number_of_results: u32,
    pub results: Vec<SearxngResult>,
    pub answers: Vec<SearxngAnswer>,
    pub corrections: Vec<SearxngCorrection>,
    pub suggestions: Vec<String>,
    pub infoboxes: Vec<SearxngInfoBox>,
    pub unresponsive_engines: Vec<Vec<String>>
    
}


#[derive(Clone, Deserialize, Debug)]
pub struct SearxngResult {
    pub template: String,

    pub title:  String,
    pub url:    String,
    pub content: String,
    pub engine: String,
    pub img_src: String,
    pub thumbnail: Option<String>,
    pub parsed_url: Option<Vec<String>>,
    pub priority: Option<String>,
    pub engines: Option<Vec<String>>,
    pub positions: Option<Vec<u32>>,
    pub score: Option<f32>,
    pub category: Option<String>,
    #[serde(rename="publishedDate")]
    pub published_date: Option<String>,
    #[serde(rename="pubdate")]
    pub pub_date: Option<String>,

    
    #[serde(flatten)]
    pub unknown: HashMap<String, serde_json::Value>


}

#[derive(Clone, Deserialize, Debug)]
pub struct SearxngAnswer {

    #[serde(flatten)]
    s: HashMap<String, serde_json::Value>
}


#[derive(Clone, Deserialize, Debug)]
pub struct SearxngCorrection {
    #[serde(flatten)]
    c: HashMap<String, serde_json::Value>
}

#[derive(Clone, Deserialize, Debug)]
pub struct SearxngInfoBox {
    url: Option<String>,
    engine: Option<String>,
    parsed_url: Option<Vec<String>>,
    img_src: String,
    thumbnail: Option<String>,
    infobox: String,
    id: String,
    content: String,
    attributes: Vec<InfoBoxAtrribute>,
    urls: Vec<SearxngUrl>,
    template: String,
    title: String,
    positions: String,
    priority: String,
    score: Option<f32>,
    engines: Option<Vec<String>>,
    #[serde(rename="publishedDate")]
    published_date: Option<String>,
    category: String,
    //length: Option<String>,
    //views: Option<String>,
    //author: Option<String>,
    //metadata: Option<String>,
    //iframe_src: Option<String>,
    //audio_src: Option<String>,
    //magnetlink: Option<String>,
    //torrentfile: Option<String>,
    //seed: Option<u32>,
    //leech: Option<u32>,
    //filesize: Option<u32>,
    //files: Option<u32>,
    //address_label: Option<String>,
    //geojson: Option<GeoJson>,
    //latitude: Option<String>,
    //longitude: Option<String>,
    //address: Option<Address>,
    //country_code: Option<String>,
    //locality: Option<String>,

    #[serde(flatten)]
    unknown: HashMap<String,serde_json::Value>
}

#[derive(Clone, Deserialize, Debug)]
pub struct InfoBoxAtrribute {
    label: String,
    value: String,
    image: Option<Vec<ImageAttribute>>
}

#[derive(Clone, Deserialize, Debug)]
pub struct SearxngUrl {
    url: String,
    title: String
}

#[derive(Clone, Deserialize, Debug)]
pub struct RelatedTopic{
    name: String,
    suggestions: Option<Vec<Suggestion>>
}
#[derive(Clone, Deserialize, Debug)]
pub struct Suggestion {
    suggestion: String
}
#[derive(Clone, Deserialize, Debug)]
pub struct ImageAttribute{
    src: String,
    alt: String
}
#[derive(Clone, Deserialize, Debug)]
pub struct SearxngSuggestion {
    #[serde(flatten)]
    s: HashMap<String, serde_json::Value>
}



#[derive(Clone, Deserialize, Debug)]
pub struct GeoJson {
    #[serde(rename = "type")]
    t: String,
    features: Vec<GeoFeature>
}


#[derive(Clone, Deserialize, Debug)]
pub struct GeoFeature{
    #[serde(flatten)]
    feature: HashMap<String, serde_json::Value>
}

#[derive(Clone, Deserialize, Debug)]
pub struct Address {
    name: String,
    road: String,
    house_number: String,
    #[serde(rename="postcode")] 
    post_code: String,
    country: String,
    country_code:String,
    locality: String
}