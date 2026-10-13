use cad_observation_types::CadObservationFailure;
use std::io::Read;

pub fn dispatch(arguments: &[String]) -> Result<Vec<u8>, CadObservationFailure> {
    match arguments {
        [command] if command == "describe" => crate::describe(),
        [command, flag, request] if command == "generate" && flag == "--request" => {
            crate::generate(&read_input(request)?)
        }
        [command, request_flag, request, batch_flag, batch]
            if command == "self-check"
                && request_flag == "--request"
                && batch_flag == "--batch" =>
        {
            crate::self_check(&read_input(request)?, &read_input(batch)?)
        }
        _ => Err(CadObservationFailure::InvalidProcessInvocation),
    }
}

fn read_input(path: &str) -> Result<Vec<u8>, CadObservationFailure> {
    if path == "-" {
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|_| CadObservationFailure::InvalidProcessInvocation)?;
        Ok(bytes)
    } else {
        std::fs::read(path).map_err(|_| CadObservationFailure::InvalidProcessInvocation)
    }
}
