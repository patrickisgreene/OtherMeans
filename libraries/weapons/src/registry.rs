pub type CountryCode = [char; 3];

pub fn weapon_ids_by_domain(
    country: &CountryCode,
) -> &'static [(&'static str, &'static [&'static str])] {
    match *country {
        ['U', 'S', 'A'] => USA,
        ['R', 'U', 'S'] => RUS,
        ['C', 'H', 'N'] => CHN,
        ['D', 'E', 'U'] => DEU,
        ['F', 'R', 'A'] => FRA,
        ['G', 'B', 'R'] => GBR,
        ['I', 'N', 'D'] => IND,
        ['I', 'S', 'R'] => ISR,
        ['I', 'T', 'A'] => ITA,
        ['J', 'P', 'N'] => JPN,
        ['K', 'O', 'R'] => KOR,
        ['N', 'L', 'D'] => NLD,
        ['S', 'W', 'E'] => SWE,
        _ => &[],
    }
}

pub fn weapon_path(country: &CountryCode, domain: &str, weapon_id: &str) -> String {
    let code: String = country.iter().collect();
    format!("data/weapons/{code}/{domain}/{weapon_id}.ron")
}

pub fn find_domain(country: &CountryCode, weapon_id: &str) -> Option<&'static str> {
    for &(domain, ids) in weapon_ids_by_domain(country) {
        if ids.contains(&weapon_id) {
            return Some(domain);
        }
    }
    None
}

const USA: &[(&str, &[&str])] = &[
    (
        "air",
        &[
            "a-10", "b-2", "b-52", "f-15", "f-16", "f-18", "f-22", "f-35",
        ],
    ),
    (
        "bomb",
        &[
            "b61-12", "b83", "blu-109", "cbu-87", "gbu-12", "gbu-28", "gbu-31", "gbu-38", "gbu-39",
            "gbu-43", "gbu-53", "gbu-57", "mk-82", "mk-84",
        ],
    ),
    (
        "land",
        &["jltv", "m109a7", "m1126", "m142", "m1a2", "m2a3", "m-atv"],
    ),
    (
        "missile",
        &[
            "agm-114", "agm-158", "agm-84", "agm-88", "aim-120", "aim-9x", "bgm-109", "lgm-30g",
            "mim-104", "ugm-133",
        ],
    ),
    (
        "sea",
        &[
            "cg-47", "cvn-68", "cvn-78", "ddg-1000", "ddg-51", "lha-6", "ssbn-726", "ssn-774",
        ],
    ),
];

const RUS: &[(&str, &[&str])] = &[
    (
        "air",
        &[
            "mig-29", "mig-31", "su-25", "su-27", "su-34", "su-57", "tu-160",
        ],
    ),
    (
        "bomb",
        &[
            "atbip",
            "betab-500shp",
            "fab-1500",
            "fab-500",
            "kab-1500lg",
            "kab-500l",
            "odab-500pmv",
            "rbk-500",
        ],
    ),
    (
        "land",
        &[
            "2s35", "bmp-2m", "bmp-3", "btr-82a", "gaz-2330", "t-14", "t-90m", "tos-1a",
        ],
    ),
    (
        "missile",
        &[
            "3m-54", "48n6", "aa-11", "aa-12", "as-17", "brahmos", "ss-26", "ss-n-26", "ss-x-30",
        ],
    ),
    (
        "sea",
        &[
            "cg-1164", "cgn-096", "cv-063", "dd-1155", "ssbn-955", "ssgn-885",
        ],
    ),
];

const CHN: &[(&str, &[&str])] = &[
    ("bomb", &["ft-7", "ls-6", "lt-2"]),
    ("land", &["pcl-181", "zbd-04a", "ztq-15", "ztz-99a"]),
    ("missile", &["css-x-20", "hq-9", "pl-10", "pl-15", "yj-18"]),
    (
        "sea",
        &["cg-101", "cv-18", "ddg-172", "ssbn-094", "ssn-093"],
    ),
];

const DEU: &[(&str, &[&str])] = &[
    ("land", &["boxer-apc", "leopard-2a7", "puma-ifv"]),
    ("missile", &["bvraam", "iris-t", "kepd-350"]),
];

const FRA: &[(&str, &[&str])] = &[
    ("bomb", &["aasm"]),
    ("land", &["leclerc"]),
    ("missile", &["bvraam", "mm40", "samp-t", "scalp-eg"]),
    ("sea", &["d-652", "fgs-751", "r-91", "ssn-suffren"]),
];

const GBR: &[(&str, &[&str])] = &[
    ("bomb", &["griffin", "paveway-iv", "stormbreaker"]),
    ("land", &["challenger-3"]),
    ("missile", &["bvraam", "scalp-eg"]),
    ("sea", &["cv-r08", "d-32", "ssn-s119"]),
];

const IND: &[(&str, &[&str])] = &[
    ("bomb", &["hsab-500", "sudarshan"]),
    ("land", &["arjun-mk-ii"]),
    ("missile", &["agni-v", "brahmos", "lr-sam"]),
    ("sea", &["cv-iac-1", "d-63"]),
];

const ISR: &[(&str, &[&str])] = &[
    ("bomb", &["spice-1000", "spice-2000"]),
    ("land", &["merkava-iv", "namer"]),
    ("missile", &["lr-sam", "python-5"]),
];

const ITA: &[(&str, &[&str])] = &[
    ("land", &["centauro-ii"]),
    ("missile", &["samp-t"]),
    ("sea", &["d-652", "fgs-751"]),
];

const JPN: &[(&str, &[&str])] = &[("land", &["type-10"]), ("sea", &["ddg-179", "ss-501"])];

const KOR: &[(&str, &[&str])] = &[
    ("land", &["k2"]),
    ("missile", &["hyunmoo-3c"]),
    ("sea", &["ddg-991"]),
];

const NLD: &[(&str, &[&str])] = &[("land", &["boxer-apc"])];

const SWE: &[(&str, &[&str])] = &[("land", &["cv9035"]), ("missile", &["kepd-350"])];
