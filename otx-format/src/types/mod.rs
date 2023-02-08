pub use crate::generated::packed;

#[cfg(test)]
mod test {
    use crate::types::packed::OpenTransaction;

    use super::*;
    use ckb_jsonrpc_types::JsonBytes;
    use molecule::prelude::*;
    use packed::OpenTransactionBuilder;

    #[test]
    fn test_serialize() {
        let builder = OpenTransactionBuilder::default();
        let opentx = builder.build();
        let opentx_bytes = opentx.as_bytes();
        let json_rpc_format = JsonBytes::from_bytes(opentx_bytes);
        println!("{:?}", opentx);
        println!("{:?}", json_rpc_format);

        let opentx_bytes = json_rpc_format.as_bytes();
        println!("{:?}", opentx_bytes);
        let opentx_rebuild = OpenTransaction::from_slice(opentx_bytes).unwrap();

        assert_eq!(opentx.as_bytes(), opentx_rebuild.as_bytes());
    }
}
