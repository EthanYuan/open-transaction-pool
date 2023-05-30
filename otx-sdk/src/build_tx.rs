use otx_format::jsonrpc_types::OpenTransaction;
use utils::config::ScriptInfo;

use anyhow::Result;
use ckb_types::packed::{self, CellOutput, OutPoint};

pub fn build_otx(
    _inputs: Vec<OutPoint>,
    _outputs: Vec<CellOutput>,
    _outputs_data: Vec<packed::Bytes>,
    _otx_script_info: &ScriptInfo,
) -> Result<OpenTransaction> {
    todo!()
}
