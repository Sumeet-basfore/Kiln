//! `kiln-bin`: main binary entrypoint for Kiln.

fn main() {
    println!("kiln {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bin_version() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.0.1");
    }
}
