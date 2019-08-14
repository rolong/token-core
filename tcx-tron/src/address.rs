use bitcoin::util::base58;
use std::str::FromStr;
use tcx_chain::{Address1 as AddressTrait, PublicKey};
use tcx_primitive::key::secp256k1::Public;

pub struct Address(String);

pub enum Error {
    InvalidBase58,
}

impl AddressTrait for Address {
    type Error = Error;
    type Public = Public;

    fn from_public(public: &Self::Public) -> core::result::Result<Address, Self::Error> {
        let bytes = public.0.public_key.to_uncompressed();
        let hash = keccak_hash::keccak(&bytes[1..]);
        let hex: Vec<u8> = [vec![0x41], hash[12..32].to_vec()].concat();
        Ok(Address(base58::check_encode_slice(&hex)))
    }
}

#[cfg(test)]
mod tests {
    use super::Address;

    #[test]
    fn tron_address() {
        let bytes = hex::decode("04DAAC763B1B3492720E404C53D323BAF29391996F7DD5FA27EF0D12F7D50D694700684A32AD97FF4C09BF9CF0B9D0AC7F0091D9C6CB8BE9BB6A1106DA557285D8").unwrap();
        //let public_key = <bitcoin::PublicKey as PublicKey>::from_slice(&bytes).unwrap();

        //        assert_eq!(Address::from_public_key(&public_key).unwrap().0, "THfuSDVRvSsjNDPFdGjMU19Ha4Kf7acotq");
    }
}
