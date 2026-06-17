use crate::tasks::MapReduceVersion;


pub trait MapReduceImplementation {
    type IntermediateType;
    type OutputType;
}

impl MapReduceImplementation for MapReduceVersion::Default {
    
}   
