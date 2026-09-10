use super::s256;

#[test]
fn s256_matches_rfc7636_appendix_b() {
    assert_eq!(
        s256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
}
