use std::cmp::min;
use std::collections::HashMap;
use crate::Chunk;

const MIN_CHUNK_SIZE: usize = 1024 * 4;
const AVG_CHUNK_SIZE: usize = 1024 * 8;
const MAX_CHUNK_SIZE: usize = 1024 * 64;

// 8KB, 4KB and 2KB masks
const MASK_S: u64 = 0b1111_1111_1111;
const MASK_L: u64 = 0b111_1111_1111;
const MASK_B: u64 = 0b11_1111_1111;

const MASK_S_LS: u64 = MASK_B << 1;
const MASK_L_LS: u64 = MASK_L << 1;
const MASK_B_LS: u64 = MASK_B << 1;

pub struct Chunker<'a> {
    buf: &'a [u8],
    records: HashMap<u64, usize>,
    last_hash: u64,
    record_last_hash: bool,
    pos: usize,
    shelved: Option<usize>
}

impl<'a> Chunker<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            pos: 0,
            records: Default::default(),
            last_hash: 0,
            record_last_hash: false,
            shelved: None
        }
    }

    fn use_record_map(&mut self, hash: u64, length: usize) -> Option<usize> {
        if self.record_last_hash {
            self.records.insert(self.last_hash, length);
        }

        if let Some(&found_length) = self.records.get(&hash) {
            self.record_last_hash = false;
            if self.pos + found_length < self.buf.len() {
                return Some(found_length);
            }
        } else {
            self.record_last_hash = true;
        }

        self.last_hash = hash;
        None
    }

    fn find_border(&mut self, buf: &[u8]) -> Option<(u64, usize)> {
        if buf.len() < MIN_CHUNK_SIZE {
            return None;
        }

        let remaining = min(MAX_CHUNK_SIZE, buf.len());
        let center = min(AVG_CHUNK_SIZE, buf.len());

        let mut breakpoint = remaining;
        let mut breakpoint_flag = false;

        let mut fingerprint: u64 = 0;
        let mut pos: usize = MIN_CHUNK_SIZE / 2;

        let mut breakpoint_gear = 0;
        let mut gear;

        for index in 1..16 {
            fingerprint = fingerprint
                .wrapping_add(GEAR[buf[MIN_CHUNK_SIZE - index] as usize] << index);
            pos += 1;
        }

        while pos < center / 2 {
            let a = pos * 2;
            gear = GEAR_LS[buf[a] as usize];
            fingerprint = (fingerprint << 2).wrapping_add(gear);
            if fingerprint & MASK_S_LS == 0 {
                return Some((gear, a));
            }
            gear = GEAR[buf[a + 1] as usize];
            fingerprint = fingerprint.wrapping_add(gear);
            if fingerprint & MASK_S == 0 {
                return Some((gear, a + 1));
            }
            pos += 1;
        }

        while pos < remaining / 2 {
            let a = pos * 2;
            gear = GEAR_LS[buf[a] as usize];
            fingerprint = (fingerprint << 2).wrapping_add(gear);
            if fingerprint & MASK_L_LS == 0 {
                return Some((gear, a));
            }
            if !breakpoint_flag && fingerprint & MASK_B_LS == 0 {
                breakpoint_flag = true;
                breakpoint = a;
                breakpoint_gear = gear;
            }

            gear = GEAR[buf[a + 1] as usize];
            fingerprint = fingerprint.wrapping_add(gear);
            if fingerprint & MASK_L == 0 {
                return Some((gear, a + 1));
            }
            if !breakpoint_flag && fingerprint & MASK_B == 0 {
                breakpoint_flag = true;
                breakpoint = a + 1;
                breakpoint_gear = gear;
            }
            pos += 1;
        }

        if pos == remaining / 2 {
            return Some((breakpoint_gear, breakpoint));
        }

        Some((breakpoint_gear, breakpoint))
    }
}

impl<'a> Iterator for Chunker<'a> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(length) = self.shelved {
            self.shelved = None;

            let chunk = Chunk::new(self.pos, length);

            self.pos += length;

            return Some(chunk);
        }

        let search_range = self.pos..self.buf.len();
        if let Some((hash, length)) = self.find_border(&self.buf[search_range]) {
            let chunk = Chunk::new(self.pos, length);

            self.pos += length;

            if let Some(length) = self.use_record_map(hash, length) {
                self.shelved = Some(length);
            }

            Some(chunk)
        } else {
            None
        }
    }
}

// Gear table taken from https://github.com/nlfiedler/fastcdc-rs
#[rustfmt::skip]
const GEAR: [u64; 256] = [
    0x3b5d3c7d207e37dc, 0x784d68ba91123086, 0xcd52880f882e7298, 0xeacf8e4e19fdcca7,
    0xc31f385dfbd1632b, 0x1d5f27001e25abe6, 0x83130bde3c9ad991, 0xc4b225676e9b7649,
    0xaa329b29e08eb499, 0xb67fcbd21e577d58, 0x0027baaada2acf6b, 0xe3ef2d5ac73c2226,
    0x0890f24d6ed312b7, 0xa809e036851d7c7e, 0xf0a6fe5e0013d81b, 0x1d026304452cec14,
    0x03864632648e248f, 0xcdaacf3dcd92b9b4, 0xf5e012e63c187856, 0x8862f9d3821c00b6,
    0xa82f7338750f6f8a, 0x1e583dc6c1cb0b6f, 0x7a3145b69743a7f1, 0xabb20fee404807eb,
    0xb14b3cfe07b83a5d, 0xb9dc27898adb9a0f, 0x3703f5e91baa62be, 0xcf0bb866815f7d98,
    0x3d9867c41ea9dcd3, 0x1be1fa65442bf22c, 0x14300da4c55631d9, 0xe698e9cbc6545c99,
    0x4763107ec64e92a5, 0xc65821fc65696a24, 0x76196c064822f0b7, 0x485be841f3525e01,
    0xf652bc9c85974ff5, 0xcad8352face9e3e9, 0x2a6ed1dceb35e98e, 0xc6f483badc11680f,
    0x3cfd8c17e9cf12f1, 0x89b83c5e2ea56471, 0xae665cfd24e392a9, 0xec33c4e504cb8915,
    0x3fb9b15fc9fe7451, 0xd7fd1fd1945f2195, 0x31ade0853443efd8, 0x255efc9863e1e2d2,
    0x10eab6008d5642cf, 0x46f04863257ac804, 0xa52dc42a789a27d3, 0xdaaadf9ce77af565,
    0x6b479cd53d87febb, 0x6309e2d3f93db72f, 0xc5738ffbaa1ff9d6, 0x6bd57f3f25af7968,
    0x67605486d90d0a4a, 0xe14d0b9663bfbdae, 0xb7bbd8d816eb0414, 0xdef8a4f16b35a116,
    0xe7932d85aaaffed6, 0x08161cbae90cfd48, 0x855507beb294f08b, 0x91234ea6ffd399b2,
    0xad70cf4b2435f302, 0xd289a97565bc2d27, 0x8e558437ffca99de, 0x96d2704b7115c040,
    0x0889bbcdfc660e41, 0x5e0d4e67dc92128d, 0x72a9f8917063ed97, 0x438b69d409e016e3,
    0xdf4fed8a5d8a4397, 0x00f41dcf41d403f7, 0x4814eb038e52603f, 0x9dafbacc58e2d651,
    0xfe2f458e4be170af, 0x4457ec414df6a940, 0x06e62f1451123314, 0xbd1014d173ba92cc,
    0xdef318e25ed57760, 0x9fea0de9dfca8525, 0x459de1e76c20624b, 0xaeec189617e2d666,
    0x126a2c06ab5a83cb, 0xb1321532360f6132, 0x65421503dbb40123, 0x2d67c287ea089ab3,
    0x6c93bff5a56bd6b6, 0x4ffb2036cab6d98d, 0xce7b785b1be7ad4f, 0xedb42ef6189fd163,
    0xdc905288703988f6, 0x365f9c1d2c691884, 0xc640583680d99bfe, 0x3cd4624c07593ec6,
    0x7f1ea8d85d7c5805, 0x014842d480b57149, 0x0b649bcb5a828688, 0xbcd5708ed79b18f0,
    0xe987c862fbd2f2f0, 0x982731671f0cd82c, 0xbaf13e8b16d8c063, 0x8ea3109cbd951bba,
    0xd141045bfb385cad, 0x2acbc1a0af1f7d30, 0xe6444d89df03bfdf, 0xa18cc771b8188ff9,
    0x9834429db01c39bb, 0x214add07fe086a1f, 0x8f07c19b1f6b3ff9, 0x56a297b1bf4ffe55,
    0x94d558e493c54fc7, 0x40bfc24c764552cb, 0x931a706f8a8520cb, 0x32229d322935bd52,
    0x2560d0f5dc4fefaf, 0x9dbcc48355969bb6, 0x0fd81c3985c0b56a, 0xe03817e1560f2bda,
    0xc1bb4f81d892b2d5, 0xb0c4864f4e28d2d7, 0x3ecc49f9d9d6c263, 0x51307e99b52ba65e,
    0x8af2b688da84a752, 0xf5d72523b91b20b6, 0x6d95ff1ff4634806, 0x562f21555458339a,
    0xc0ce47f889336346, 0x487823e5089b40d8, 0xe4727c7ebc6d9592, 0x5a8f7277e94970ba,
    0xfca2f406b1c8bb50, 0x5b1f8a95f1791070, 0xd304af9fc9028605, 0x5440ab7fc930e748,
    0x312d25fbca2ab5a1, 0x10f4a4b234a4d575, 0x90301d55047e7473, 0x3b6372886c61591e,
    0x293402b77c444e06, 0x451f34a4d3e97dd7, 0x3158d814d81bc57b, 0x034942425b9bda69,
    0xe2032ff9e532d9bb, 0x62ae066b8b2179e5, 0x9545e10c2f8d71d8, 0x7ff7483eb2d23fc0,
    0x00945fcebdc98d86, 0x8764bbbe99b26ca2, 0x1b1ec62284c0bfc3, 0x58e0fcc4f0aa362b,
    0x5f4abefa878d458d, 0xfd74ac2f9607c519, 0xa4e3fb37df8cbfa9, 0xbf697e43cac574e5,
    0x86f14a3f68f4cd53, 0x24a23d076f1ce522, 0xe725cd8048868cc8, 0xbf3c729eb2464362,
    0xd8f6cd57b3cc1ed8, 0x6329e52425541577, 0x62aa688ad5ae1ac0, 0x0a242566269bf845,
    0x168b1a4753aca74b, 0xf789afefff2e7e3c, 0x6c3362093b6fccdb, 0x4ce8f50bd28c09b2,
    0x006a2db95ae8aa93, 0x975b0d623c3d1a8c, 0x18605d3935338c5b, 0x5bb6f6136cad3c71,
    0x0f53a20701f8d8a6, 0xab8c5ad2e7e93c67, 0x40b5ac5127acaa29, 0x8c7bf63c2075895f,
    0x78bd9f7e014a805c, 0xb2c9e9f4f9c8c032, 0xefd6049827eb91f3, 0x2be459f482c16fbd,
    0xd92ce0c5745aaa8c, 0x0aaa8fb298d965b9, 0x2b37f92c6c803b15, 0x8c54a5e94e0f0e78,
    0x95f9b6e90c0a3032, 0xe7939faa436c7874, 0xd16bfe8f6a8a40c9, 0x44982b86263fd2fa,
    0xe285fb39f984e583, 0x779a8df72d7619d3, 0xf2d79a8de8d5dd1e, 0xd1037354d66684e2,
    0x004c82a4e668a8e5, 0x31d40a7668b044e6, 0xd70578538bd02c11, 0xdb45431078c5f482,
    0x977121bb7f6a51ad, 0x73d5ccbd34eff8dd, 0xe437a07d356e17cd, 0x47b2782043c95627,
    0x9fb251413e41d49a, 0xccd70b60652513d3, 0x1c95b31e8a1b49b2, 0xcae73dfd1bcb4c1b,
    0x34d98331b1f5b70f, 0x784e39f22338d92f, 0x18613d4a064df420, 0xf1d8dae25f0bcebe,
    0x33f77c15ae855efc, 0x3c88b3b912eb109c, 0x956a2ec96bafeea5, 0x1aa005b5e0ad0e87,
    0x5500d70527c4bb8e, 0xe36c57196421cc44, 0x13c4d286cc36ee39, 0x5654a23d818b2a81,
    0x77b1dc13d161abdc, 0x734f44de5f8d5eb5, 0x60717e174a6c89a2, 0xd47d9649266a211e,
    0x5b13a4322bb69e90, 0xf7669609f8b5fc3c, 0x21e6ac55bedcdac9, 0x9b56b62b61166dea,
    0xf48f66b939797e9c, 0x35f332f9c0e6ae9a, 0xcc733f6a9a878db0, 0x3da161e41cc108c2,
    0xb7d74ae535914d51, 0x4d493b0b11d36469, 0xce264d1dfba9741a, 0xa9d1f2dc7436dc06,
    0x70738016604c2a27, 0x231d36e96e93f3d5, 0x7666881197838d19, 0x4a2a83090aaad40c,
    0xf1e761591668b35d, 0x7363236497f730a7, 0x301080e37379dd4d, 0x502dea2971827042,
    0xc2c5eb858f32625f, 0x786afb9edfafbdff, 0xdaee0d868490b2a4, 0x617366b3268609f6,
    0xae0e35a0fe46173e, 0xd1a07de93e824f11, 0x079b8b115ea4cca8, 0x93a99274558faebb,
    0xfb1e6e22e08a03b3, 0xea635fdba3698dd0, 0xcf53659328503a5c, 0xcde3b31e6fd5d780,
    0x8e3e4221d3614413, 0xef14d0d86bf1a22c, 0xe1d830d3f16c5ddb, 0xaabd2b2a451504e1
];

// Gear table, shifted one bit to the left, taken from https://github.com/nlfiedler/fastcdc-rs
#[rustfmt::skip]
const GEAR_LS: [u64; 256] = [
    0x76ba78fa40fc6fb8, 0xf09ad1752224610c, 0x9aa5101f105ce530, 0xd59f1c9c33fb994e,
    0x863e70bbf7a2c656, 0x3abe4e003c4b57cc, 0x062617bc7935b322, 0x89644acedd36ec92,
    0x54653653c11d6932, 0x6cff97a43caefab0, 0x004f7555b4559ed6, 0xc7de5ab58e78444c,
    0x1121e49adda6256e, 0x5013c06d0a3af8fc, 0xe14dfcbc0027b036, 0x3a04c6088a59d828,
    0x070c8c64c91c491e, 0x9b559e7b9b257368, 0xebc025cc7830f0ac, 0x10c5f3a70438016c,
    0x505ee670ea1edf14, 0x3cb07b8d839616de, 0xf4628b6d2e874fe2, 0x57641fdc80900fd6,
    0x629679fc0f7074ba, 0x73b84f1315b7341e, 0x6e07ebd23754c57c, 0x9e1770cd02befb30,
    0x7b30cf883d53b9a6, 0x37c3f4ca8857e458, 0x28601b498aac63b2, 0xcd31d3978ca8b932,
    0x8ec620fd8c9d254a, 0x8cb043f8cad2d448, 0xec32d80c9045e16e, 0x90b7d083e6a4bc02,
    0xeca579390b2e9fea, 0x95b06a5f59d3c7d2, 0x54dda3b9d66bd31c, 0x8de90775b822d01e,
    0x79fb182fd39e25e2, 0x137078bc5d4ac8e2, 0x5cccb9fa49c72552, 0xd86789ca0997122a,
    0x7f7362bf93fce8a2, 0xaffa3fa328be432a, 0x635bc10a6887dfb0, 0x4abdf930c7c3c5a4,
    0x21d56c011aac859e, 0x8de090c64af59008, 0x4a5b8854f1344fa6, 0xb555bf39cef5eaca,
    0xd68f39aa7b0ffd76, 0xc613c5a7f27b6e5e, 0x8ae71ff7543ff3ac, 0xd7aafe7e4b5ef2d0,
    0xcec0a90db21a1494, 0xc29a172cc77f7b5c, 0x6f77b1b02dd60828, 0xbdf149e2d66b422c,
    0xcf265b0b555ffdac, 0x102c3975d219fa90, 0x0aaa0f7d6529e116, 0x22469d4dffa73364,
    0x5ae19e96486be604, 0xa51352eacb785a4e, 0x1cab086fff9533bc, 0x2da4e096e22b8080,
    0x1113779bf8cc1c82, 0xbc1a9ccfb924251a, 0xe553f122e0c7db2e, 0x8716d3a813c02dc6,
    0xbe9fdb14bb14872e, 0x01e83b9e83a807ee, 0x9029d6071ca4c07e, 0x3b5f7598b1c5aca2,
    0xfc5e8b1c97c2e15e, 0x88afd8829bed5280, 0x0dcc5e28a2246628, 0x7a2029a2e7752598,
    0xbde631c4bdaaeec0, 0x3fd41bd3bf950a4a, 0x8b3bc3ced840c496, 0x5dd8312c2fc5accc,
    0x24d4580d56b50796, 0x62642a646c1ec264, 0xca842a07b7680246, 0x5acf850fd4113566,
    0xd9277feb4ad7ad6c, 0x9ff6406d956db31a, 0x9cf6f0b637cf5a9e, 0xdb685dec313fa2c6,
    0xb920a510e07311ec, 0x6cbf383a58d23108, 0x8c80b06d01b337fc, 0x79a8c4980eb27d8c,
    0xfe3d51b0baf8b00a, 0x029085a9016ae292, 0x16c93796b5050d10, 0x79aae11daf3631e0,
    0xd30f90c5f7a5e5e0, 0x304e62ce3e19b058, 0x75e27d162db180c6, 0x1d4621397b2a3774,
    0xa28208b7f670b95a, 0x559783415e3efa60, 0xcc889b13be077fbe, 0x43198ee370311ff2,
    0x3068853b60387376, 0x4295ba0ffc10d43e, 0x1e0f83363ed67ff2, 0xad452f637e9ffcaa,
    0x29aab1c9278a9f8e, 0x817f8498ec8aa596, 0x2634e0df150a4196, 0x64453a64526b7aa4,
    0x4ac1a1ebb89fdf5e, 0x3b798906ab2d376c, 0x1fb038730b816ad4, 0xc0702fc2ac1e57b4,
    0x83769f03b12565aa, 0x61890c9e9c51a5ae, 0x7d9893f3b3ad84c6, 0xa260fd336a574cbc,
    0x15e56d11b5094ea4, 0xebae4a477236416c, 0xdb2bfe3fe8c6900c, 0xac5e42aaa8b06734,
    0x819c8ff11266c68c, 0x90f047ca113681b0, 0xc8e4f8fd78db2b24, 0xb51ee4efd292e174,
    0xf945e80d639176a0, 0xb63f152be2f220e0, 0xa6095f3f92050c0a, 0xa88156ff9261ce90,
    0x625a4bf794556b42, 0x21e949646949aaea, 0x20603aaa08fce8e6, 0x76c6e510d8c2b23c,
    0x5268056ef8889c0c, 0x8a3e6949a7d2fbae, 0x62b1b029b0378af6, 0x06928484b737b4d2,
    0xc4065ff3ca65b376, 0xc55c0cd71642f3ca, 0x2a8bc2185f1ae3b0, 0xffee907d65a47f80,
    0x0128bf9d7b931b0c, 0x0ec9777d3364d944, 0x363d8c4509817f86, 0xb1c1f989e1546c56,
    0xbe957df50f1a8b1a, 0xfae9585f2c0f8a32, 0x49c7f66fbf197f52, 0x7ed2fc87958ae9ca,
    0x0de2947ed1e99aa6, 0x49447a0ede39ca44, 0xce4b9b00910d1990, 0x7e78e53d648c86c4,
    0xb1ed9aaf67983db0, 0xc653ca484aa82aee, 0xc554d115ab5c3580, 0x14484acc4d37f08a,
    0x2d16348ea7594e96, 0xef135fdffe5cfc78, 0xd866c41276df99b6, 0x99d1ea17a5181364,
    0x00d45b72b5d15526, 0x2eb61ac4787a3518, 0x30c0ba726a6718b6, 0xb76dec26d95a78e2,
    0x1ea7440e03f1b14c, 0x5718b5a5cfd278ce, 0x816b58a24f595452, 0x18f7ec7840eb12be,
    0xf17b3efc029500b8, 0x6593d3e9f3918064, 0xdfac09304fd723e6, 0x57c8b3e90582df7a,
    0xb259c18ae8b55518, 0x15551f6531b2cb72, 0x566ff258d900762a, 0x18a94bd29c1e1cf0,
    0x2bf36dd218146064, 0xcf273f5486d8f0e8, 0xa2d7fd1ed5148192, 0x8930570c4c7fa5f4,
    0xc50bf673f309cb06, 0xef351bee5aec33a6, 0xe5af351bd1abba3c, 0xa206e6a9accd09c4,
    0x00990549ccd151ca, 0x63a814ecd16089cc, 0xae0af0a717a05822, 0xb68a8620f18be904,
    0x2ee24376fed4a35a, 0xe7ab997a69dff1ba, 0xc86f40fa6adc2f9a, 0x8f64f0408792ac4e,
    0x3f64a2827c83a934, 0x99ae16c0ca4a27a6, 0x392b663d14369364, 0x95ce7bfa37969836,
    0x69b3066363eb6e1e, 0xf09c73e44671b25e, 0x30c27a940c9be840, 0xe3b1b5c4be179d7c,
    0x67eef82b5d0abdf8, 0x7911677225d62138, 0x2ad45d92d75fdd4a, 0x35400b6bc15a1d0e,
    0xaa01ae0a4f89771c, 0xc6d8ae32c8439888, 0x2789a50d986ddc72, 0xaca9447b03165502,
    0xef63b827a2c357b8, 0xe69e89bcbf1abd6a, 0xc0e2fc2e94d91344, 0xa8fb2c924cd4423c,
    0xb6274864576d3d20, 0xeecd2c13f16bf878, 0x43cd58ab7db9b592, 0x36ad6c56c22cdbd4,
    0xe91ecd7272f2fd38, 0x6be665f381cd5d34, 0x98e67ed5350f1b60, 0x7b42c3c839821184,
    0x6fae95ca6b229aa2, 0x9a92761623a6c8d2, 0x9c4c9a3bf752e834, 0x53a3e5b8e86db80c,
    0xe0e7002cc098544e, 0x463a6dd2dd27e7aa, 0xeccd10232f071a32, 0x945506121555a818,
    0xe3cec2b22cd166ba, 0xe6c646c92fee614e, 0x602101c6e6f3ba9a, 0xa05bd452e304e084,
    0x858bd70b1e64c4be, 0xf0d5f73dbf5f7bfe, 0xb5dc1b0d09216548, 0xc2e6cd664d0c13ec,
    0x5c1c6b41fc8c2e7c, 0xa340fbd27d049e22, 0x0f371622bd499950, 0x275324e8ab1f5d76,
    0xf63cdc45c1140766, 0xd4c6bfb746d31ba0, 0x9ea6cb2650a074b8, 0x9bc7663cdfabaf00,
    0x1c7c8443a6c28826, 0xde29a1b0d7e34458, 0xc3b061a7e2d8bbb6, 0x557a56548a2a09c2
];
