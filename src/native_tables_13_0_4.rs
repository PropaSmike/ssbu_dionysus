const MANIFEST_SHA256: &str = "F70C97D2BDCD00C2E81559F5ABD0871E3C550B240D2005AB2E6085957A9D7613";
const BACKEND_CAPACITY: usize = 256;
const FIRST_NATIVE_BACKEND_KIND: i32 = 121;
const LAST_NATIVE_BACKEND_KIND: i32 = 255;
const EXPECTED_OPCODES: &[ExpectedOpcode] = &[
    ExpectedOpcode {
        offset: 0x2c9b68,
        opcode: 0x7101d51f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x2c9b6c,
        opcode: 0x540000a8,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x2c9b70,
        opcode: 0xd0021389,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x2c9b74,
        opcode: 0x91070129,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x2c9b78,
        opcode: 0xf8687920,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32724c,
        opcode: 0x7101d53f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x327250,
        opcode: 0x540000a8,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x327254,
        opcode: 0x900210ab,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x327258,
        opcode: 0x9107016b,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32725c,
        opcode: 0xf8697969,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32aa6c,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32aa70,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32aa74,
        opcode: 0xb0021088,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32aa78,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32aa7c,
        opcode: 0xf86a7908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32ab40,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32ab44,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32ab48,
        opcode: 0xb0021088,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32ab4c,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32ab50,
        opcode: 0xf86a7908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32ac50,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32ac54,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32ac58,
        opcode: 0xb0021088,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32ac5c,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32ac60,
        opcode: 0xf86a7908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32ad58,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32ad5c,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32ad60,
        opcode: 0xb0021088,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32ad64,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32ad68,
        opcode: 0xf86a7908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32ae28,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32ae2c,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32ae30,
        opcode: 0xb0021088,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32ae34,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32ae38,
        opcode: 0xf86a7908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32afa8,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32afac,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32afb0,
        opcode: 0xb0021088,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32afb4,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32afb8,
        opcode: 0xf86a7908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32b520,
        opcode: 0x7101d53f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32b524,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32b528,
        opcode: 0x9002108a,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32b52c,
        opcode: 0x9107014a,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32b530,
        opcode: 0xf8697957,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32eacc,
        opcode: 0x7101d51f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32ead0,
        opcode: 0x540000a8,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32ead4,
        opcode: 0xb002106a,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32ead8,
        opcode: 0x9107014a,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32eadc,
        opcode: 0xf868794a,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x32f5d8,
        opcode: 0x7101d5bf,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x32f5dc,
        opcode: 0x540000a8,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x32f5e0,
        opcode: 0x90021068,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x32f5e4,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x32f5e8,
        opcode: 0xf86d7908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x330e14,
        opcode: 0xf002105b,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x330e18,
        opcode: 0x9107037b,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x331164,
        opcode: 0x7101d41f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x33116c,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x331170,
        opcode: 0xf8607b6a,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x3315b8,
        opcode: 0x7101d5ff,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x3315c0,
        opcode: 0x54000068,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x3316dc,
        opcode: 0x7101d41f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x3316e8,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33176c,
        opcode: 0x7101d5df,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x331774,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x3318cc,
        opcode: 0x7101d63f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x3318d4,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33199c,
        opcode: 0x7101d65f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x3319ac,
        opcode: 0x54000068,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x331aa8,
        opcode: 0x7101d63f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x331ab0,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x331f3c,
        opcode: 0x7101d57f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x331f44,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33913c,
        opcode: 0x7101d41f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x339148,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x3391e8,
        opcode: 0x7101d63f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x3391f0,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33a4f0,
        opcode: 0xb002101b,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x33a4f4,
        opcode: 0x9107037b,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x33b0f8,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x33b0fc,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33b100,
        opcode: 0xf86a7b77,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x33b1c0,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x33b1c4,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33b2a4,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x33b2a8,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33b3a4,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x33b3a8,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33b454,
        opcode: 0x7101d55f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x33b458,
        opcode: 0x54000048,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33f454,
        opcode: 0x7101d53f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x33f458,
        opcode: 0x540000a8,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x33f45c,
        opcode: 0x90020fea,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x33f460,
        opcode: 0x9107014a,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x33f464,
        opcode: 0xf869d94a,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x3ae16c,
        opcode: 0x7101d69f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x3ae170,
        opcode: 0x54001e48,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x3ae174,
        opcode: 0xb0020c68,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x3ae178,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x3ae17c,
        opcode: 0xf874d904,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x3ae6dc,
        opcode: 0x7101d69f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x3ae6e0,
        opcode: 0x54001e48,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x3ae6e4,
        opcode: 0xb0020c68,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x3ae6e8,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x3ae6ec,
        opcode: 0xf874d904,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x60c16c,
        opcode: 0x90024ba9,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x60c170,
        opcode: 0x91388129,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x60c174,
        opcode: 0xf8687922,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x60d314,
        opcode: 0xf0024b8a,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x60d318,
        opcode: 0x9138814a,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x60d320,
        opcode: 0xf869d942,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x618590,
        opcode: 0x7101d51f,
        label: "fighter_name_upper bound",
    },
    ExpectedOpcode {
        offset: 0x618594,
        opcode: 0x540000a8,
        label: "fighter_name_upper bound branch",
    },
    ExpectedOpcode {
        offset: 0x618598,
        opcode: 0x90024b49,
        label: "fighter_name_upper ADRP",
    },
    ExpectedOpcode {
        offset: 0x61859c,
        opcode: 0x9129c129,
        label: "fighter_name_upper ADD",
    },
    ExpectedOpcode {
        offset: 0x6185a0,
        opcode: 0xf8687922,
        label: "fighter_name_upper first consumer",
    },
    ExpectedOpcode {
        offset: 0x63fe94,
        opcode: 0x7101d6df,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x63fe9c,
        opcode: 0x540000a8,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x63fea0,
        opcode: 0x9001f7e8,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x63fea4,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x63fea8,
        opcode: 0xf8767908,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x68d530,
        opcode: 0x7101781f,
        label: "fighter_class_descriptor bound",
    },
    ExpectedOpcode {
        offset: 0x68d538,
        opcode: 0xd0026068,
        label: "fighter_class_descriptor ADRP",
    },
    ExpectedOpcode {
        offset: 0x68d53c,
        opcode: 0x913f4108,
        label: "fighter_class_descriptor ADD",
    },
    ExpectedOpcode {
        offset: 0x68d540,
        opcode: 0xf860d900,
        label: "fighter_class_descriptor first consumer",
    },
    ExpectedOpcode {
        offset: 0x68dd58,
        opcode: 0x7101d51f,
        label: "fighter_name_upper bound",
    },
    ExpectedOpcode {
        offset: 0x68dd5c,
        opcode: 0x540000e8,
        label: "fighter_name_upper bound branch",
    },
    ExpectedOpcode {
        offset: 0x68dd60,
        opcode: 0xf0024789,
        label: "fighter_name_upper ADRP",
    },
    ExpectedOpcode {
        offset: 0x68dd64,
        opcode: 0x9129c129,
        label: "fighter_name_upper ADD",
    },
    ExpectedOpcode {
        offset: 0x68dd70,
        opcode: 0xf8687922,
        label: "fighter_name_upper first consumer",
    },
    ExpectedOpcode {
        offset: 0x6e6db0,
        opcode: 0x7101d51f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x6e6db4,
        opcode: 0x54001908,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x6e6db8,
        opcode: 0xf00244c9,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x6e6dbc,
        opcode: 0x91074129,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x6e6dc0,
        opcode: 0xf868d93a,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x6eae10,
        opcode: 0x7101d51f,
        label: "fighter_name_upper bound",
    },
    ExpectedOpcode {
        offset: 0x6eae14,
        opcode: 0x540001e8,
        label: "fighter_name_upper bound branch",
    },
    ExpectedOpcode {
        offset: 0x6eae18,
        opcode: 0xd00244a9,
        label: "fighter_name_upper ADRP",
    },
    ExpectedOpcode {
        offset: 0x6eae1c,
        opcode: 0x9129c129,
        label: "fighter_name_upper ADD",
    },
    ExpectedOpcode {
        offset: 0x6eae20,
        opcode: 0xf8687922,
        label: "fighter_name_upper first consumer",
    },
    ExpectedOpcode {
        offset: 0x6eb000,
        opcode: 0x7101d51f,
        label: "fighter_name_upper bound",
    },
    ExpectedOpcode {
        offset: 0x6eb004,
        opcode: 0x540001e8,
        label: "fighter_name_upper bound branch",
    },
    ExpectedOpcode {
        offset: 0x6eb008,
        opcode: 0xb00244a9,
        label: "fighter_name_upper ADRP",
    },
    ExpectedOpcode {
        offset: 0x6eb00c,
        opcode: 0x9129c129,
        label: "fighter_name_upper ADD",
    },
    ExpectedOpcode {
        offset: 0x6eb010,
        opcode: 0xf8687922,
        label: "fighter_name_upper first consumer",
    },
    ExpectedOpcode {
        offset: 0x70a45c,
        opcode: 0xb001f18e,
        label: "fighter_param_hash_compact ADRP",
    },
    ExpectedOpcode {
        offset: 0x70a460,
        opcode: 0x913161ce,
        label: "fighter_param_hash_compact ADD",
    },
    ExpectedOpcode {
        offset: 0x70a47c,
        opcode: 0xf86879d2,
        label: "fighter_param_hash_compact first consumer",
    },
    ExpectedOpcode {
        offset: 0x70a548,
        opcode: 0xf101791f,
        label: "fighter_param_hash_compact bound",
    },
    ExpectedOpcode {
        offset: 0xba4478,
        opcode: 0x90021ee8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0xba447c,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0xba4488,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0xba4e84,
        opcode: 0x90021ee9,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0xba4e88,
        opcode: 0x91388129,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0xba4e8c,
        opcode: 0xf868d922,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0xfbe2b8,
        opcode: 0x7101d51f,
        label: "fighter_name_upper bound",
    },
    ExpectedOpcode {
        offset: 0xfbe2bc,
        opcode: 0x540000a8,
        label: "fighter_name_upper bound branch",
    },
    ExpectedOpcode {
        offset: 0xfbe2c0,
        opcode: 0xd001fe09,
        label: "fighter_name_upper ADRP",
    },
    ExpectedOpcode {
        offset: 0xfbe2c4,
        opcode: 0x9129c129,
        label: "fighter_name_upper ADD",
    },
    ExpectedOpcode {
        offset: 0xfbe2c8,
        opcode: 0xf8687922,
        label: "fighter_name_upper first consumer",
    },
    ExpectedOpcode {
        offset: 0x1492948,
        opcode: 0x7101d53f,
        label: "fighter_name_lower bound",
    },
    ExpectedOpcode {
        offset: 0x149294c,
        opcode: 0x540000a8,
        label: "fighter_name_lower bound branch",
    },
    ExpectedOpcode {
        offset: 0x1492950,
        opcode: 0xd001d76a,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x1492954,
        opcode: 0x9138814a,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x1492958,
        opcode: 0xf869d942,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x14977cc,
        opcode: 0x7101d53f,
        label: "fighter_name_lower bound",
    },
    ExpectedOpcode {
        offset: 0x14977d0,
        opcode: 0x540000a8,
        label: "fighter_name_lower bound branch",
    },
    ExpectedOpcode {
        offset: 0x14977d4,
        opcode: 0xb001d74a,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x14977d8,
        opcode: 0x9138814a,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x14977dc,
        opcode: 0xf869d942,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x166ee98,
        opcode: 0x7101d67f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x166eea0,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x166eea4,
        opcode: 0xb0017668,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x166eea8,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x166eeac,
        opcode: 0xf87a791b,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x166f028,
        opcode: 0x7101d67f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x166f030,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x166f034,
        opcode: 0x90017668,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x166f038,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x166f03c,
        opcode: 0xf87a791b,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x166f1b8,
        opcode: 0x7101d67f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x166f1c0,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x166f1c4,
        opcode: 0x90017668,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x166f1c8,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x166f1cc,
        opcode: 0xf87a791b,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x166f348,
        opcode: 0x7101d67f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x166f350,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x166f354,
        opcode: 0x90017668,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x166f358,
        opcode: 0x91070108,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x166f35c,
        opcode: 0xf87a791b,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df498,
        opcode: 0xb001bd08,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df49c,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17df4a4,
        opcode: 0xf861d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df61c,
        opcode: 0xb001bd09,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df620,
        opcode: 0x91388129,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17df624,
        opcode: 0xf861d922,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df688,
        opcode: 0xb001bd08,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df68c,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17df690,
        opcode: 0xf861d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df6a4,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df6a8,
        opcode: 0x54000fc8,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df6ac,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df6b0,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df6b4,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df6bc,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df6c0,
        opcode: 0x54000fa8,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df6c4,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df6c8,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df6cc,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df6d4,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df6d8,
        opcode: 0x54000f88,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df6dc,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df6e0,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df6e4,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df6ec,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df6f0,
        opcode: 0x54000f68,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df6f4,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df6f8,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df6fc,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df710,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df714,
        opcode: 0x54000ee8,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df718,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df71c,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df720,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df728,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df72c,
        opcode: 0x54000ec8,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df730,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df734,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df738,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df76c,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df770,
        opcode: 0x54000d48,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df774,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df778,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df77c,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df784,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df788,
        opcode: 0x54000d28,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df78c,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df790,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df794,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df79c,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df7a0,
        opcode: 0x54000d08,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df7a4,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df7a8,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df7ac,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df7b4,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df7b8,
        opcode: 0x54000ce8,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df7bc,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df7c0,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df7c4,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df7cc,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df7d0,
        opcode: 0x54000cc8,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df7d4,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df7d8,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df7dc,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df7e4,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df7e8,
        opcode: 0x54000ca8,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df7ec,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df7f0,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df7f4,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df820,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df824,
        opcode: 0x54000b68,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df828,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df82c,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df830,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df838,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df83c,
        opcode: 0x54000b48,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df840,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df844,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df848,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df850,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df854,
        opcode: 0x54000b28,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df858,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df85c,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df860,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17df868,
        opcode: 0x7101d43f,
        label: "fighter_name_title bound",
    },
    ExpectedOpcode {
        offset: 0x17df86c,
        opcode: 0x54000b08,
        label: "fighter_name_title bound branch",
    },
    ExpectedOpcode {
        offset: 0x17df870,
        opcode: 0xd001bd08,
        label: "fighter_name_title ADRP",
    },
    ExpectedOpcode {
        offset: 0x17df874,
        opcode: 0x91074108,
        label: "fighter_name_title ADD",
    },
    ExpectedOpcode {
        offset: 0x17df878,
        opcode: 0xf861d902,
        label: "fighter_name_title first consumer",
    },
    ExpectedOpcode {
        offset: 0x17dfcdc,
        opcode: 0x7101d69f,
        label: "fighter_name_lower bound",
    },
    ExpectedOpcode {
        offset: 0x17dfce0,
        opcode: 0x540000a8,
        label: "fighter_name_lower bound branch",
    },
    ExpectedOpcode {
        offset: 0x17dfce4,
        opcode: 0xb001bd08,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17dfce8,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17dfcec,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e0320,
        opcode: 0x7101d6bf,
        label: "fighter_name_lower bound",
    },
    ExpectedOpcode {
        offset: 0x17e0324,
        opcode: 0x540000a8,
        label: "fighter_name_lower bound branch",
    },
    ExpectedOpcode {
        offset: 0x17e0328,
        opcode: 0x9001bd08,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e032c,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e0330,
        opcode: 0xf875d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e03dc,
        opcode: 0x7101d6bf,
        label: "fighter_name_lower bound",
    },
    ExpectedOpcode {
        offset: 0x17e03e0,
        opcode: 0x540000e8,
        label: "fighter_name_lower bound branch",
    },
    ExpectedOpcode {
        offset: 0x17e03e4,
        opcode: 0x9001bd08,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e03e8,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e03f4,
        opcode: 0xf875d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e4bac,
        opcode: 0x7101d69f,
        label: "fighter_name_lower bound",
    },
    ExpectedOpcode {
        offset: 0x17e4bb0,
        opcode: 0x540000a8,
        label: "fighter_name_lower bound branch",
    },
    ExpectedOpcode {
        offset: 0x17e4bb4,
        opcode: 0x9001bce8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e4bb8,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e4bbc,
        opcode: 0xf874d916,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e74c8,
        opcode: 0xb001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e74cc,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e74d4,
        opcode: 0xf873d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e8be4,
        opcode: 0x9001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e8be8,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e8bec,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e8c04,
        opcode: 0x9001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e8c08,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e8c0c,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e8d2c,
        opcode: 0x9001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e8d30,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e8d34,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e8d4c,
        opcode: 0x9001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e8d50,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e8d54,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e8dd8,
        opcode: 0x9001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e8ddc,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e8de0,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e8df8,
        opcode: 0x9001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e8dfc,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e8e00,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e8fdc,
        opcode: 0x9001bcc8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e8fe0,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e8fe4,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e91a0,
        opcode: 0xf001bca8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e91a4,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e91a8,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e9a54,
        opcode: 0xf001bca8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e9a58,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e9a5c,
        opcode: 0xf862d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e9b7c,
        opcode: 0xf001bca8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e9b80,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e9b84,
        opcode: 0xf862d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17e9d40,
        opcode: 0xf001bca8,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17e9d44,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17e9d48,
        opcode: 0xf875d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17f0038,
        opcode: 0x7101d69f,
        label: "fighter_name_lower bound",
    },
    ExpectedOpcode {
        offset: 0x17f003c,
        opcode: 0x540000a8,
        label: "fighter_name_lower bound branch",
    },
    ExpectedOpcode {
        offset: 0x17f0040,
        opcode: 0x9001bc88,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17f0044,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17f0048,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17f0f30,
        opcode: 0x9001bc88,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17f0f34,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17f0f38,
        opcode: 0xf8697902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x17f1140,
        opcode: 0xf001bc68,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x17f1144,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x17f1148,
        opcode: 0xf8697902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x2b7d868,
        opcode: 0x7101d51f,
        label: "fighter_kind_hash bound",
    },
    ExpectedOpcode {
        offset: 0x2b7d870,
        opcode: 0x54000088,
        label: "fighter_kind_hash bound branch",
    },
    ExpectedOpcode {
        offset: 0x2b7d874,
        opcode: 0xd000cde9,
        label: "fighter_kind_hash ADRP",
    },
    ExpectedOpcode {
        offset: 0x2b7d878,
        opcode: 0x91070129,
        label: "fighter_kind_hash ADD",
    },
    ExpectedOpcode {
        offset: 0x2b7d87c,
        opcode: 0xf8687929,
        label: "fighter_kind_hash first consumer",
    },
    ExpectedOpcode {
        offset: 0x33ac034,
        opcode: 0xb0008d88,
        label: "article_agent_creator_0 adrp",
    },
    ExpectedOpcode {
        offset: 0x33ac038,
        opcode: 0x911f9108,
        label: "article_agent_creator_0 add",
    },
    ExpectedOpcode {
        offset: 0x33ac03c,
        opcode: 0xb8767908,
        label: "article_agent_creator_0 load",
    },
    ExpectedOpcode {
        offset: 0x33ac040,
        opcode: 0x7101d51f,
        label: "article_agent_creator_0 bound",
    },
    ExpectedOpcode {
        offset: 0x33ac044,
        opcode: 0x54fffca8,
        label: "article_agent_creator_0 branch",
    },
    ExpectedOpcode {
        offset: 0x33acf6c,
        opcode: 0xb0008d88,
        label: "article_agent_creator_1 adrp",
    },
    ExpectedOpcode {
        offset: 0x33acf70,
        opcode: 0x911f9108,
        label: "article_agent_creator_1 add",
    },
    ExpectedOpcode {
        offset: 0x33acf74,
        opcode: 0xb8767908,
        label: "article_agent_creator_1 load",
    },
    ExpectedOpcode {
        offset: 0x33acf78,
        opcode: 0x7101d51f,
        label: "article_agent_creator_1 bound",
    },
    ExpectedOpcode {
        offset: 0x33acf7c,
        opcode: 0x54fffce8,
        label: "article_agent_creator_1 branch",
    },
    ExpectedOpcode {
        offset: 0x33adecc,
        opcode: 0x90008d88,
        label: "article_agent_creator_2 adrp",
    },
    ExpectedOpcode {
        offset: 0x33aded0,
        opcode: 0x911f9108,
        label: "article_agent_creator_2 add",
    },
    ExpectedOpcode {
        offset: 0x33aded4,
        opcode: 0xb8767908,
        label: "article_agent_creator_2 load",
    },
    ExpectedOpcode {
        offset: 0x33aded8,
        opcode: 0x7101d51f,
        label: "article_agent_creator_2 bound",
    },
    ExpectedOpcode {
        offset: 0x33adedc,
        opcode: 0x54fffce8,
        label: "article_agent_creator_2 branch",
    },
    ExpectedOpcode {
        offset: 0x33aee2c,
        opcode: 0xf0008d68,
        label: "article_agent_creator_3 adrp",
    },
    ExpectedOpcode {
        offset: 0x33aee30,
        opcode: 0x911f9108,
        label: "article_agent_creator_3 add",
    },
    ExpectedOpcode {
        offset: 0x33aee34,
        opcode: 0xb8767908,
        label: "article_agent_creator_3 load",
    },
    ExpectedOpcode {
        offset: 0x33aee38,
        opcode: 0x7101d51f,
        label: "article_agent_creator_3 bound",
    },
    ExpectedOpcode {
        offset: 0x33aee3c,
        opcode: 0x54fffce8,
        label: "article_agent_creator_3 branch",
    },
    ExpectedOpcode {
        offset: 0x33b65ec,
        opcode: 0xf0008d3a,
        label: "article_param_builder adrp",
    },
    ExpectedOpcode {
        offset: 0x33b65f0,
        opcode: 0x911f935a,
        label: "article_param_builder add",
    },
    ExpectedOpcode {
        offset: 0x341b3c4,
        opcode: 0xb000db28,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x341b3c8,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x341b3cc,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x341ddc8,
        opcode: 0xf000db08,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x341ddcc,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x341ddd0,
        opcode: 0xf877d902,
        label: "fighter_name_lower first consumer",
    },
    ExpectedOpcode {
        offset: 0x341f14c,
        opcode: 0xb000db08,
        label: "fighter_name_lower ADRP",
    },
    ExpectedOpcode {
        offset: 0x341f150,
        opcode: 0x91388108,
        label: "fighter_name_lower ADD",
    },
    ExpectedOpcode {
        offset: 0x341f154,
        opcode: 0xf874d902,
        label: "fighter_name_lower first consumer",
    },
];
