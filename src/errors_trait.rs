use log;
use rmcp::ErrorData;
use std::error::Error as Error;

pub trait LogError {

    fn log_error<T,E>(&self, error:E, custom_message: Option<String>) -> Result<T,E> 
    where 
        E: Error
    {
        log::debug!("log trait activated");
        
        let message: String = match custom_message {
            Some(m) => format!("{}\n{}",m,error),
            None => error.to_string()
        };

        log::error!("{}", message);

        Err(error)

    }

}