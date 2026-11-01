//! Iris masks for the base-avatar eyes. Base eyes ship one colour texture and no mask, so the eye
//! colour tinted nothing; each bundled mask is black where the iris is and white elsewhere, the
//! same convention as a wearable's own mask. Keyed by the texture's content hash, so a redeployed
//! texture stops matching instead of getting a misaligned mask. Regenerate with
//! `tools/eye_masks/generate.py`.

/// The bundled mask for a base eye texture, and whether the shader paints the iris with the picked
/// colour (eyes whose iris is a solid black disc) instead of multiplying it.
pub fn base_eye_mask(texture_hash: &str) -> Option<(&'static str, bool)> {
    BASE_EYE_MASKS
        .iter()
        .find(|(hash, _)| *hash == texture_hash)
        .map(|(_, path)| (*path, MASK_REPLACE.contains(&texture_hash)))
}

const MASK_REPLACE: [&str; 2] = [
    "bafkreieo2imr2lvlfiesq4ollve3oflwwlmfmjg5pjytz46kc5gontdjxm",
    "bafkreidv6rr4jf2w2igal3wjkscomdrworp5gr7ud7uwbaczayfiktbsne",
];

const BASE_EYE_MASKS: [(&str, &str); 35] = [
    (
        "bafkreihse7i7mqabinjq4aydv3kngpxkoownwx2z3ul5ko3vz3zcn6qzii",
        "res://assets/avatar/eye_masks/eyes_00_mask.png",
    ),
    (
        "bafkreidkftg5hs2nl5qh5jkpqv25ljxrnjdfzzr6hxsjxrvjgpnsyxbbda",
        "res://assets/avatar/eye_masks/eyes_01_mask.png",
    ),
    (
        "bafkreidbeafduyssl7y2gdjoxq3ouagn6beivty4v6eovpl5iz2z72jeqm",
        "res://assets/avatar/eye_masks/eyes_02_mask.png",
    ),
    (
        "bafkreieo2imr2lvlfiesq4ollve3oflwwlmfmjg5pjytz46kc5gontdjxm",
        "res://assets/avatar/eye_masks/eyes_03_mask.png",
    ),
    (
        "bafkreiddfvmi3kvxgjqcwjev6ot4rkeswbwv7nt5pmpr5du3dywpbiiegq",
        "res://assets/avatar/eye_masks/eyes_04_mask.png",
    ),
    (
        "bafkreih7ndgrj5tytwnohttgg3h5m3j4bylgq5s35j5fznwnzsxvl7cc3q",
        "res://assets/avatar/eye_masks/eyes_05_mask.png",
    ),
    (
        "bafkreigmp6kksur4nln6fsnt4bx3nwmjtztw22ti47zpqxlhihw7x3s5bm",
        "res://assets/avatar/eye_masks/eyes_06_mask.png",
    ),
    (
        "bafkreihseyrap76samsaksp7qgeu64gzf53n74dl4flgsxbz3zykgajv3a",
        "res://assets/avatar/eye_masks/eyes_07_mask.png",
    ),
    (
        "bafkreiekdf7ryiigiairg2zz3vc6xcddlcrzotygtupyafjzv4ahvunjaq",
        "res://assets/avatar/eye_masks/eyes_08_mask.png",
    ),
    (
        "bafkreiby5x6ddjdrrqfpm7oexja6vecfcz5bbbs6qe6xzmqgglam5yvhle",
        "res://assets/avatar/eye_masks/eyes_09_mask.png",
    ),
    (
        "bafkreib6pel36fnphe6qsykogdhtnvje65fmjjigkjwvaybnc3dcxfd5mu",
        "res://assets/avatar/eye_masks/eyes_10_mask.png",
    ),
    (
        "bafkreifwflrbyaq4rlisyrzgmjunm4ah55fznzt4mvcadgt5tp3iriv6nu",
        "res://assets/avatar/eye_masks/eyes_11_mask.png",
    ),
    (
        "bafkreifzn5vsmpmu46zow4agsccsaimdywxr5a5cc4lrq3ldepkypfse54",
        "res://assets/avatar/eye_masks/eyes_12_mask.png",
    ),
    (
        "bafkreidk3znxzfmamkof5qoisoff4cfgjickunz26mvhq57i2dzpgxslra",
        "res://assets/avatar/eye_masks/eyes_13_mask.png",
    ),
    (
        "bafkreic5ileafhebzefbcojhoc63yt5m7chpq7jsvng54pvxxtcyl6dg6e",
        "res://assets/avatar/eye_masks/eyes_14_mask.png",
    ),
    (
        "bafkreifdvhnxvn746f4cou2v6yhhnndp3agutlhl623curtmeleik45pme",
        "res://assets/avatar/eye_masks/eyes_15_mask.png",
    ),
    (
        "bafkreiav4wpfer7zvlpx5trgsv2ywfh4w75djd5mx5oors4ey2ppt73zfa",
        "res://assets/avatar/eye_masks/eyes_16_mask.png",
    ),
    (
        "bafkreictmrdsxaxzifrldbu2cdgtpxjtwwcx2hxpksgrhqi2tavdopcjua",
        "res://assets/avatar/eye_masks/eyes_17_mask.png",
    ),
    (
        "bafkreidyabyqbtpgafx6bhruw2qayjqnkycvs567u5kdf42onmrioqropa",
        "res://assets/avatar/eye_masks/eyes_18_mask.png",
    ),
    (
        "bafkreid6zw7p6eawronfrfjyayshoom4wafzobohx76m3lbqmg44pe7e54",
        "res://assets/avatar/eye_masks/eyes_19_mask.png",
    ),
    (
        "bafkreihtn32uuiickc4cpt7mk24mqo6yjm72jj6rhawbolcenf2eofqc6u",
        "res://assets/avatar/eye_masks/eyes_20_mask.png",
    ),
    (
        "bafkreiajbrkhqu5t5sww4wnfg4vwfeztmit7uiconjow4x2up2o2agjygq",
        "res://assets/avatar/eye_masks/eyes_21_mask.png",
    ),
    (
        "bafkreidlxtqmsajwzkpncthl3benw3boid4gjhavj2t7g5sskzqydsoerm",
        "res://assets/avatar/eye_masks/eyes_22_mask.png",
    ),
    (
        "bafkreihm3s5xcauc6i256xnywwssnodcvtrs6z3454itsf2ph63e3tx7iq",
        "res://assets/avatar/eye_masks/f_eyes_00_mask.png",
    ),
    (
        "bafkreihyo4jsqjsgfwdwucdfyvbkkww7mgkc4dsypbarxpwuhgpygxjmaq",
        "res://assets/avatar/eye_masks/f_eyes_01_mask.png",
    ),
    (
        "bafkreidm6bs6vixgdi5obils34ujmhittgf43dwie6gjzlrc644gzkhjsu",
        "res://assets/avatar/eye_masks/f_eyes_02_mask.png",
    ),
    (
        "bafkreidv6rr4jf2w2igal3wjkscomdrworp5gr7ud7uwbaczayfiktbsne",
        "res://assets/avatar/eye_masks/f_eyes_03_mask.png",
    ),
    (
        "bafkreigmm5zfnzu4egbiajlx4iivfy36stx243zytg5fvk2jfgmlka7zci",
        "res://assets/avatar/eye_masks/f_eyes_04_mask.png",
    ),
    (
        "bafkreibtcouz565jspqfwoissec6w3cjlo4zxfc73xihj2clhw2nsaadoi",
        "res://assets/avatar/eye_masks/f_eyes_05_mask.png",
    ),
    (
        "bafkreiho5cykajxijt3bss7h6s7pobrspzfave2wu6gfj2ndzreynz5bum",
        "res://assets/avatar/eye_masks/f_eyes_06_mask.png",
    ),
    (
        "bafkreihsngpwvcdmgrcj5tmmfzwpppwmrsc6r3b2d5zvy4ud4zcaxdvjw4",
        "res://assets/avatar/eye_masks/f_eyes_07_mask.png",
    ),
    (
        "bafkreicqdnzkngyjm25eyd2mgfzk4rmoaeakd3bjcyh6i277q2rstktxza",
        "res://assets/avatar/eye_masks/f_eyes_08_mask.png",
    ),
    (
        "bafkreif7kkkztcw2ts6ijo7pqc7k2sgc5kvab46t566hjtd7slxjicxste",
        "res://assets/avatar/eye_masks/f_eyes_09_mask.png",
    ),
    (
        "bafkreifin5nam6vkl25mugq7rtjaslxmkc2bxjksr56teirlpl3vv2ahhy",
        "res://assets/avatar/eye_masks/f_eyes_10_mask.png",
    ),
    (
        "bafkreicdjq7mi4cs2lcnzbiwnkxg3pbhzlie4yethzvcekbekqvto32ewu",
        "res://assets/avatar/eye_masks/f_eyes_11_mask.png",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_black_iris_eyes_paint_and_the_rest_multiply() {
        let (path, replace) =
            base_eye_mask("bafkreieo2imr2lvlfiesq4ollve3oflwwlmfmjg5pjytz46kc5gontdjxm").unwrap();
        assert!(replace && path.ends_with("eyes_03_mask.png"));
        let (_, replace) =
            base_eye_mask("bafkreihse7i7mqabinjq4aydv3kngpxkoownwx2z3ul5ko3vz3zcn6qzii").unwrap();
        assert!(!replace);
        assert!(base_eye_mask("unknown").is_none());
    }

    #[test]
    fn every_replace_entry_has_a_mask() {
        for hash in MASK_REPLACE {
            assert!(base_eye_mask(hash).is_some());
        }
    }
}
