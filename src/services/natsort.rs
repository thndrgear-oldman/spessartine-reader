use std::cmp::Ordering;

pub fn natcmp(a: &str, b: &str) -> Ordering {
    let mut a = a;
    let mut b = b;
    loop {
        let ad = a.find(|c: char| c.is_ascii_digit());
        let bd = b.find(|c: char| c.is_ascii_digit());

        match a[..ad.unwrap_or(a.len())].cmp(&b[..bd.unwrap_or(b.len())]) {
            Ordering::Equal => {}
            other => return other,
        }

        match (ad, bd) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ap), Some(bp)) => {
                let an: String = a[ap..].chars().take_while(|c| c.is_ascii_digit()).collect();
                let bn: String = b[bp..].chars().take_while(|c| c.is_ascii_digit()).collect();
                let at = an.trim_start_matches('0');
                let bt = bn.trim_start_matches('0');
                match at.len().cmp(&bt.len()).then(at.cmp(bt)) {
                    Ordering::Equal => {}
                    other => return other,
                }
                a = &a[ap + an.len()..];
                b = &b[bp + bn.len()..];
            }
        }
    }
}
