use crate::CadFailure;

#[derive(Debug, Eq, PartialEq)]
pub struct CadFailureApi;

#[derive(Debug, Eq, PartialEq)]
pub struct FailureApiRequest;

pub fn cad_failure_api(_request: FailureApiRequest) -> Result<CadFailureApi, CadFailure> {
    Ok(CadFailureApi)
}
