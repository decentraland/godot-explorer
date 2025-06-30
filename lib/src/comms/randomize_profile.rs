use rand::distributions::Alphanumeric;
use rand::Rng;

use crate::comms::profile::{
    AvatarColor, AvatarColor3, AvatarEmote, AvatarSnapshots, AvatarWireFormat, SerializedProfile,
    UserProfile,
};

impl UserProfile {
    pub fn randomize() -> Self {
        Self {
            base_url: "https://peer.decentraland.org/content/contents/".to_owned(),
            version: 1,
            content: SerializedProfile::randomize(),
        }
    }
}

impl SerializedProfile {
    pub fn randomize() -> Self {
        let mut rng = rand::thread_rng();

        // Generate random name (8-15 alphanumeric characters)
        let name_length = rng.gen_range(8..=15);
        let name: String = (0..name_length)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();

        // Generate random description
        let descriptions = vec![
            "Crypto enthusiast and DeFi explorer",
            "Building the future of Web3",
            "Digital artist and NFT creator",
            "Blockchain developer and researcher",
            "Decentralized finance advocate",
            "Metaverse architect and builder",
            "Web3 community contributor",
            "Virtual world explorer",
            "Digital fashion collector",
            "Decentraland citizen",
            "", // Some profiles might have no description
        ];
        let description = descriptions[rng.gen_range(0..descriptions.len())].to_string();

        // Use AvatarWireFormat::randomize() for the avatar
        let avatar = AvatarWireFormat::randomize();

        // Create profile with randomized name, description, avatar, and address
        Self {
            name,
            description,
            avatar,
            has_claimed_name: Some(false),
            has_connected_web3: Some(false),
            ..Self::default()
        }
    }
}

impl AvatarWireFormat {
    pub fn randomize() -> Self {
        let mut rng = rand::thread_rng();

        // Avatar configurations from paste.txt
        let avatar_configs = vec![
            // Config 1: Female with cat eyes
            ("urn:decentraland:off-chain:base-avatars:BaseFemale", vec![
                "urn:decentraland:matic:collections-v2:0x696156d1d58ecb6bcd0d8268fda2900032233d36:0:43",
                "urn:decentraland:off-chain:base-avatars:f_eyebrows_06",
                "urn:decentraland:ethereum:collections-v1:halloween_2020:hwn_2020_cat_eyes:1161",
                "urn:decentraland:matic:collections-v2:0x168ea124480bee0caa6673a8b761ae02b98b758d:0:21",
                "urn:decentraland:matic:collections-v2:0x563e2081b3cd716ed76fc0993b7e49939cb342a5:0:47",
                "urn:decentraland:matic:collections-v2:0x1aeb7d9536193a3a25c74d462ec2dc88da9e50dd:4:421249166674228746791672110734681729275580381602196445017243910178",
                "urn:decentraland:matic:collections-v2:0x06350901317394fa9d581d10b4f72910020f337e:0:39",
                "urn:decentraland:matic:collections-v2:0xa8a4e51633c50dcb81e64a16a04b71b69c813868:0:10",
                "urn:decentraland:matic:collections-v2:0x5b26e7cc59e72633887e539575d754d034886133:1:105312291668557186697918027683670432318895095400549111254310977538",
                "urn:decentraland:matic:collections-v2:0x377c46eb4e3714d7c32fe669121c37ecf980c59f:1:105312291668557186697918027683670432318895095400549111254310977604",
                "urn:decentraland:matic:collections-v2:0xc6a4a95e66af77ee3d2a8a6821aeab044a1c2f55:0:1",
                "urn:decentraland:matic:collections-v2:0x0bcf8746b13b8ef0a3923c8911eb0e8046af67ff:0:16",
                "urn:decentraland:matic:collections-v2:0x5826113f948fe30978b822b210c7399dd4c0342a:1:105312291668557186697918027683670432318895095400549111254310977562",
                "urn:decentraland:matic:collections-v2:0xf239a55ebc3f42f164c47e3ee450d62b8926f4f0:0:6"
            ], vec![]),
            // Config 2: Male with cat eyes and pumpkin mouth
            ("urn:decentraland:off-chain:base-avatars:BaseMale", vec![
                "urn:decentraland:matic:collections-v2:0x4ea1cfe9e8ca641fbe362469f79029a0eaa6a996:0:1",
                "urn:decentraland:matic:collections-v2:0xded1e53d7a43ac1844b66c0ca0f02627eb42e16d:7:737186041679900306885426193785693026232265667803843778780176842775",
                "urn:decentraland:matic:collections-v2:0x80ec22f149a3f4727ba1ab91ddcefe9860ed4808:0:409",
                "urn:decentraland:matic:collections-v2:0x3f0ba00d6a98db91f92e5e04accc9735a6fc562d:3:315936875005671560093754083051011296956685286201647333762932932618",
                "urn:decentraland:matic:collections-v2:0x8cc5f22eec7b03c6a183499d7bb50679097ddfb6:0:10",
                "urn:decentraland:matic:collections-v2:0x083230402677cad61d7a0cc040dc8a08cfeb809e:6:631873750011343120187508166102022593913370572403294667525865865251",
                "urn:decentraland:matic:collections-v2:0xd1b84ae24b95f290211a6da61cde8d1b2647c711:0:5",
                "urn:decentraland:matic:collections-v2:0xbad1c910568230c92c223b55564cf689928ec1b6:0:4",
                "urn:decentraland:matic:collections-v2:0x5b7bd3e12f7d1461fc0ad370cd12402049b4b0d9:0:25",
                "urn:decentraland:matic:collections-v2:0x324cb2c654be51c6ad6c76a3e022cabe49cc4e46:4:421249166674228746791672110734681729275580381602196445017243910158",
                "urn:decentraland:off-chain:base-avatars:eyebrows_06",
                "urn:decentraland:ethereum:collections-v1:halloween_2020:hwn_2020_cat_eyes:409",
                "urn:decentraland:ethereum:collections-v1:halloween_2020:hwn_2020_pumpkin_mouth:394",
                "urn:decentraland:matic:collections-v2:0xcbd17d94f5be12fff27566c23888d6936b8e70b8:6:631873750011343120187508166102022593913370572403294667525865865262"
            ], vec!["eyewear", "hands_wear", "facial_hair"]),
            // Config 3: Simple Male avatar
            ("urn:decentraland:off-chain:base-avatars:BaseMale", vec![
                "urn:decentraland:off-chain:base-avatars:yellow_tshirt",
                "urn:decentraland:off-chain:base-avatars:soccer_pants",
                "urn:decentraland:off-chain:base-avatars:comfy_sport_sandals",
                "urn:decentraland:off-chain:base-avatars:keanu_hair",
                "urn:decentraland:off-chain:base-avatars:granpa_beard"
            ], vec![]),
            // Config 4: Male with force render
            ("urn:decentraland:off-chain:base-avatars:BaseMale", vec![
                "urn:decentraland:matic:collections-v2:0xa83c8951dd73843bf5f7e9936e72a345a3e79874:8:842498333348457493583344221469363458551160763204392890034487820295",
                "urn:decentraland:matic:collections-v2:0x89dd5ee70e4fa4400b02bac1145f5260bb827a24:0:1",
                "urn:decentraland:matic:collections-v2:0x83a600dfb82a4806f60f5ee5bf02c306639fe385:0:24",
                "urn:decentraland:matic:collections-v2:0xaf26e33ccea26e697e71b005499f820b95821c04:0:7",
                "urn:decentraland:matic:collections-v2:0xa83c8951dd73843bf5f7e9936e72a345a3e79874:7:737186041679900306885426193785693026232265667803843778780176842765",
                "urn:decentraland:matic:collections-v2:0xfb1d9d5dbb92f2dccc841bd3085081bb1bbeb04d:13:1369059791691243427072934359887715620145636240207138446306042707992",
                "urn:decentraland:matic:collections-v2:0xd62cb20c1fc76962aae30e7067babdf66463ffe3:0:6",
                "urn:decentraland:matic:collections-v2:0x844a933934fba88434dfade0b04b1d211e92d7c4:0:57",
                "urn:decentraland:matic:collections-v2:0x7d65d7ca3d44814c697aea3a1db45da330546e7b:0:55",
                "urn:decentraland:matic:collections-v2:0x3da9e56ce30dc83f6415ce35acdcc71c236e1829:2:210624583337114373395836055367340864637790190801098222508621955117",
                "urn:decentraland:matic:collections-v2:0xb055cc2916bf8857ad1ae19b0c8a4d128180c4a9:0:115",
                "urn:decentraland:matic:collections-v2:0x2929bbb4f18b40ac52a7f0b91629c695e3f96504:1:105312291668557186697918027683670432318895095400549111254310977567",
                "urn:decentraland:matic:collections-v2:0x34f266ed68b877dd98ee2697f09bc0481be828bd:0:90",
                "urn:decentraland:matic:collections-v2:0xf3df68b5748f1955f68b4fefda3f65b2e0250325:0:100",
                "urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
                "urn:decentraland:matic:collections-v2:0xac3b666704ec025b2e59f22249830a07b6fb9573:0:30"
            ], vec!["helmet", "lower_body", "tiara", "hands_wear", "feet", "upper_body"]),
            // Config 5: Male with mask
            ("urn:decentraland:off-chain:base-avatars:BaseMale", vec![
                "urn:decentraland:off-chain:base-avatars:Thunder_earring",
                "urn:decentraland:off-chain:base-avatars:eyebrows_06",
                "urn:decentraland:off-chain:base-avatars:eyes_22",
                "urn:decentraland:off-chain:base-avatars:horseshoe_beard",
                "urn:decentraland:off-chain:base-avatars:modern_hair",
                "urn:decentraland:off-chain:base-avatars:mouth_09",
                "urn:decentraland:matic:collections-v2:0x0dc28547b88100eb6b3f3890f0501607aa5dd6be:0:3202",
                "urn:decentraland:matic:collections-v2:0xbf83965191065487db0644812649d5238435c723:1:105312291668557186697918027683670432318895095400549111254310978934"
            ], vec![]),
            // Config 6: Cybersoldier
            ("urn:decentraland:off-chain:base-avatars:BaseMale", vec![
                "urn:decentraland:off-chain:base-avatars:mouth_03",
                "urn:decentraland:off-chain:base-avatars:eyes_08",
                "urn:decentraland:off-chain:base-avatars:eyebrows_00",
                "urn:decentraland:off-chain:base-avatars:chin_beard",
                "urn:decentraland:off-chain:base-avatars:cool_hair",
                "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_boots_feet:25",
                "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_helmet:29",
                "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_leggings_lower_body:34",
                "urn:decentraland:ethereum:collections-v1:cybermike_cybersoldier_set:cybersoldier_torso_upper_body:35"
            ], vec![]),
            // Config 7: Female casual
            ("urn:decentraland:off-chain:base-avatars:BaseFemale", vec![
                "urn:decentraland:off-chain:base-avatars:colored_sweater",
                "urn:decentraland:off-chain:base-avatars:f_african_leggins",
                "urn:decentraland:off-chain:base-avatars:citycomfortableshoes",
                "urn:decentraland:off-chain:base-avatars:hair_undere",
                "urn:decentraland:off-chain:base-avatars:black_sun_glasses",
                "urn:decentraland:off-chain:base-avatars:f_mouth_05"
            ], vec![]),
            // Config 8: Female school uniform
            ("urn:decentraland:off-chain:base-avatars:BaseFemale", vec![
                "urn:decentraland:off-chain:base-avatars:school_shirt",
                "urn:decentraland:off-chain:base-avatars:f_school_skirt",
                "urn:decentraland:off-chain:base-avatars:Moccasin",
                "urn:decentraland:off-chain:base-avatars:hair_anime_01",
                "urn:decentraland:off-chain:base-avatars:f_eyes_08",
                "urn:decentraland:off-chain:base-avatars:blue_star_earring"
            ], vec![]),
        ];

        // Pick a random avatar configuration
        let config_idx = rng.gen_range(0..avatar_configs.len());
        let (body_shape, wearables, force_render) = &avatar_configs[config_idx];

        // Random eye color
        let eyes = Some(AvatarColor {
            color: AvatarColor3 {
                r: rng.gen_range(0.0..1.0),
                g: rng.gen_range(0.0..1.0),
                b: rng.gen_range(0.0..1.0),
            },
        });

        // Random hair color (more natural range)
        let hair = Some(AvatarColor {
            color: AvatarColor3 {
                r: rng.gen_range(0.1..1.0),
                g: rng.gen_range(0.0..0.8),
                b: rng.gen_range(0.0..0.6),
            },
        });

        // Random skin color (natural skin tone range)
        let skin = Some(AvatarColor {
            color: AvatarColor3 {
                r: rng.gen_range(0.6..1.0),
                g: rng.gen_range(0.4..0.95),
                b: rng.gen_range(0.3..0.9),
            },
        });

        // Random snapshots - extended list for more variety
        let snapshot_ids = vec![
            (
                "bafkreigxesh5owgy4vreca65nh33zqw7br6haokkltmzg3mn22g5whcfbq",
                "bafkreibykc3l7ai5z5zik7ypxlqetgtmiepr42al6jcn4yovdgezycwa2y",
            ),
            (
                "bafkreidzk6zms72eeciw7gdvkxq5zpekz7ceqs5qhgmfso3e6q24skr6wa",
                "bafkreidyaysvsyof3f2s2k7ufmhkksm6lsawebg7bcdqlnzimwumu5qz7e",
            ),
            (
                "bafkreih4usuqlcxaklfvpojpwcmyoitgwugrfwqhzn5lm6ajfgn2swqfsi",
                "bafkreihznohs6c5ye5wcqnixzayo7g4ezxejkpfr5klol25clhsib2f56e",
            ),
            (
                "bafkreie3gtdpfqvb5uqfsvufkz6x5ms5vxzzgnxogh5pph7r7qpw3v2pgu",
                "bafkreidm2xnfr2qoev3vlmn5mt3x6tt5sbsyzlezw4htqcvgcqvrbhzywu",
            ),
            (
                "bafkreie64rp6lc3nnxh3mpvt4kkwvfrvdavzxgfmb73gnjt3hlwfghp5iq",
                "bafkreiezr3giw7ktjksklllllnzuru7xqf4rpe4p7u6xm2hxnvqhfb2pui",
            ),
        ];
        let (body_snapshot, face_snapshot) = snapshot_ids[rng.gen_range(0..snapshot_ids.len())];

        Self {
            emotes: Some(Self::random_emotes(&mut rng)),
            body_shape: Some(body_shape.to_string()),
            wearables: wearables.iter().map(|s| s.to_string()).collect(),
            snapshots: Some(AvatarSnapshots {
                body: body_snapshot.to_string(),
                face256: face_snapshot.to_string(),
                body_url: None,
                face_url: None,
            }),
            eyes,
            hair,
            skin,
            force_render: Some(force_render.iter().map(|s| s.to_string()).collect()),
        }
    }

    fn random_emotes(rng: &mut impl Rng) -> Vec<AvatarEmote> {
        // Common emote URNs
        let available_emotes = vec![
            "urn:decentraland:matic:collections-v2:0x1b559817181633db1246da7dc3722c31a034d5cc:0:0",
            "urn:decentraland:matic:collections-v2:0x1b559817181633db1246da7dc3722c31a034d5cc:0:1",
            "urn:decentraland:matic:collections-v2:0x1b559817181633db1246da7dc3722c31a034d5cc:0:2",
            "urn:decentraland:matic:collections-v2:0x1b559817181633db1246da7dc3722c31a034d5cc:0:3",
            "urn:decentraland:matic:collections-v2:0x1b559817181633db1246da7dc3722c31a034d5cc:0:4",
            "urn:decentraland:matic:collections-v2:0x574f64ac2e7ed325798ebe623edd27fe32df0b37:0:0",
            "urn:decentraland:matic:collections-v2:0x574f64ac2e7ed325798ebe623edd27fe32df0b37:0:1",
            "urn:decentraland:matic:collections-v2:0x574f64ac2e7ed325798ebe623edd27fe32df0b37:0:2",
        ];

        // Randomly select 3-6 emotes
        let num_emotes = rng.gen_range(3..=6);
        let mut selected_emotes = Vec::new();
        let mut used_indices = std::collections::HashSet::new();

        for slot in 0..num_emotes {
            let mut idx;
            loop {
                idx = rng.gen_range(0..available_emotes.len());
                if used_indices.insert(idx) {
                    break;
                }
            }

            selected_emotes.push(AvatarEmote {
                slot: slot as u32,
                urn: available_emotes[idx].to_string(),
            });
        }

        selected_emotes
    }
}
