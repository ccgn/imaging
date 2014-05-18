use std::slice;
use std::io::IoResult;
use std::default::Default;

static MAX_SEGMENTS: u8 = 4;
static NUM_DCT_TOKENS: u8 = 12;

//Prediction modes
static DC_PRED: i8 = 0;
static V_PRED: i8 = 1;
static H_PRED: i8 = 2;
static TM_PRED: i8 = 3;
static B_PRED: i8 = 4;

static B_DC_PRED: i8 = 0;
static B_TM_PRED: i8 = 1;
static B_VE_PRED: i8 = 2;
static B_HE_PRED: i8 = 3;
static B_LD_PRED: i8 = 4;
static B_RD_PRED: i8 = 5;
static B_VR_PRED: i8 = 6;
static B_VL_PRED: i8 = 7;
static B_HD_PRED: i8 = 8;
static B_HU_PRED: i8 = 9;

type Prob = u8;

static SEGMENT_ID_TREE: [i8, ..6] = [2, 4, -0, -1, -2, -3];

//Section 11.2
//Tree for determining the keyframe luma intra prediction modes:
static KEYFRAME_YMODE_TREE: [i8, ..8] = [-B_PRED, 2, 4, 6, -DC_PRED, -V_PRED, -H_PRED, -TM_PRED];

//Default probabilities for decoding the keyframe luma modes
static KEYFRAME_YMODE_PROBS: [Prob, ..4] = [145, 156, 163, 128];

//Tree for determining the keyframe B_PRED mode:
static KEYFRAME_BPRED_MODE_TREE: [i8, ..18] = [
	-B_DC_PRED, 2,
	-B_TM_PRED, 4,
	-B_VE_PRED, 6,
	8, 12,
	-B_HE_PRED, 10,
	-B_RD_PRED, -B_VR_PRED,
        -B_LD_PRED, 14,
	-B_VL_PRED, 16,
	-B_HD_PRED, -B_HU_PRED
];

//Probabilites for the BPRED_MODE_TREE
static KEYFRAME_BPRED_MODE_PROBS: [[[u8, ..9], ..10], ..10] = [
	[
		[ 231, 120,  48,  89, 115, 113, 120, 152, 112],
		[ 152, 179,  64, 126, 170, 118,  46,  70,  95],
		[ 175,  69, 143,  80,  85,  82,  72, 155, 103],
		[  56,  58,  10, 171, 218, 189,  17,  13, 152],
		[ 144,  71,  10,  38, 171, 213, 144,  34,  26],
		[ 114,  26,  17, 163,  44, 195,  21,  10, 173],
		[ 121,  24,  80, 195,  26,  62,  44,  64,  85],
		[ 170,  46,  55,  19, 136, 160,  33, 206,  71],
		[  63,  20,   8, 114, 114, 208,  12,   9, 226],
		[  81,  40,  11,  96, 182,  84,  29,  16,  36]
     	],
	[
		[ 134, 183,  89, 137,  98, 101, 106, 165, 148],
		[  72, 187, 100, 130, 157, 111,  32,  75,  80],
		[  66, 102, 167,  99,  74,  62,  40, 234, 128],
		[  41,  53,   9, 178, 241, 141,  26,   8, 107],
		[ 104,  79,  12,  27, 217, 255,  87,  17,   7],
		[  74,  43,  26, 146,  73, 166,  49,  23, 157],
		[  65,  38, 105, 160,  51,  52,  31, 115, 128],
		[  87,  68,  71,  44, 114,  51,  15, 186,  23],
		[  47,  41,  14, 110, 182, 183,  21,  17, 194],
		[  66,  45,  25, 102, 197, 189,  23,  18,  22]
	],
	[
		[  88,  88, 147, 150,  42,  46,  45, 196, 205],
		[  43,  97, 183, 117,  85,  38,  35, 179,  61],
		[  39,  53, 200,  87,  26,  21,  43, 232, 171],
		[  56,  34,  51, 104, 114, 102,  29,  93,  77],
		[ 107,  54,  32,  26,  51,   1,  81,  43,  31],
		[  39,  28,  85, 171,  58, 165,  90,  98,  64],
		[  34,  22, 116, 206,  23,  34,  43, 166,  73],
		[  68,  25, 106,  22,  64, 171,  36, 225, 114],
		[  34,  19,  21, 102, 132, 188,  16,  76, 124],
		[  62,  18,  78,  95,  85,  57,  50,  48,  51]
	],
	[
		[ 193, 101,  35, 159, 215, 111,  89,  46, 111],
		[  60, 148,  31, 172, 219, 228,  21,  18, 111],
		[ 112, 113,  77,  85, 179, 255,  38, 120, 114],
		[  40,  42,   1, 196, 245, 209,  10,  25, 109],
		[ 100,  80,   8,  43, 154,   1,  51,  26,  71],
		[  88,  43,  29, 140, 166, 213,  37,  43, 154],
		[  61,  63,  30, 155,  67,  45,  68,   1, 209],
		[ 142,  78,  78,  16, 255, 128,  34, 197, 171],
		[  41,  40,   5, 102, 211, 183,   4,   1, 221],
		[  51,  50,  17, 168, 209, 192,  23,  25,  82]
	],
	[
		[ 125,  98,  42,  88, 104,  85, 117, 175,  82],
		[  95,  84,  53,  89, 128, 100, 113, 101,  45],
		[  75,  79, 123,  47,  51, 128,  81, 171,   1],
		[  57,  17,   5,  71, 102,  57,  53,  41,  49],
		[ 115,  21,   2,  10, 102, 255, 166,  23,   6],
		[  38,  33,  13, 121,  57,  73,  26,   1,  85],
		[  41,  10,  67, 138,  77, 110,  90,  47, 114],
		[ 101,  29,  16,  10,  85, 128, 101, 196,  26],
		[  57,  18,  10, 102, 102, 213,  34,  20,  43],
		[ 117,  20,  15,  36, 163, 128,  68,   1,  26]
	],
	[
		[ 138,  31,  36, 171,  27, 166,  38,  44, 229],
		[  67,  87,  58, 169,  82, 115,  26,  59, 179],
		[  63,  59,  90, 180,  59, 166,  93,  73, 154],
		[  40,  40,  21, 116, 143, 209,  34,  39, 175],
		[  57,  46,  22,  24, 128,   1,  54,  17,  37],
		[  47,  15,  16, 183,  34, 223,  49,  45, 183],
		[  46,  17,  33, 183,   6,  98,  15,  32, 183],
		[  65,  32,  73, 115,  28, 128,  23, 128, 205],
		[  40,   3,   9, 115,  51, 192,  18,   6, 223],
		[  87,  37,   9, 115,  59,  77,  64,  21,  47]
	],
	[
		[ 104,  55,  44, 218,   9,  54,  53, 130, 226],
		[  64,  90,  70, 205,  40,  41,  23,  26,  57],
		[  54,  57, 112, 184,   5,  41,  38, 166, 213],
		[  30,  34,  26, 133, 152, 116,  10,  32, 134],
		[  75,  32,  12,  51, 192, 255, 160,  43,  51],
		[  39,  19,  53, 221,  26, 114,  32,  73, 255],
		[  31,   9,  65, 234,   2,  15,   1, 118,  73],
		[  88,  31,  35,  67, 102,  85,  55, 186,  85],
		[  56,  21,  23, 111,  59, 205,  45,  37, 192],
		[  55,  38,  70, 124,  73, 102,   1,  34,  98]
	],
	[
		[ 102,  61,  71,  37,  34,  53,  31, 243, 192],
		[  69,  60,  71,  38,  73, 119,  28, 222,  37],
		[  68,  45, 128,  34,   1,  47,  11, 245, 171],
		[  62,  17,  19,  70, 146,  85,  55,  62,  70],
		[  75,  15,   9,   9,  64, 255, 184, 119,  16],
		[  37,  43,  37, 154, 100, 163,  85, 160,   1],
		[  63,   9,  92, 136,  28,  64,  32, 201,  85],
		[  86,   6,  28,   5,  64, 255,  25, 248,   1],
		[  56,   8,  17, 132, 137, 255,  55, 116, 128],
		[  58,  15,  20,  82, 135,  57,  26, 121,  40]
	],
	[
		[ 164,  50,  31, 137, 154, 133,  25,  35, 218],
		[  51, 103,  44, 131, 131, 123,  31,   6, 158],
		[  86,  40,  64, 135, 148, 224,  45, 183, 128],
		[  22,  26,  17, 131, 240, 154,  14,   1, 209],
		[  83,  12,  13,  54, 192, 255,  68,  47,  28],
		[  45,  16,  21,  91,  64, 222,   7,   1, 197],
		[  56,  21,  39, 155,  60, 138,  23, 102, 213],
		[  85,  26,  85,  85, 128, 128,  32, 146, 171],
		[  18,  11,   7,  63, 144, 171,   4,   4, 246],
		[  35,  27,  10, 146, 174, 171,  12,  26, 128]
	],
	[
		[ 190,  80,  35,  99, 180,  80, 126,  54,  45],
		[  85, 126,  47,  87, 176,  51,  41,  20,  32],
		[ 101,  75, 128, 139, 118, 146, 116, 128,  85],
		[  56,  41,  15, 176, 236,  85,  37,   9,  62],
		[ 146,  36,  19,  30, 171, 255,  97,  27,  20],
		[  71,  30,  17, 119, 118, 255,  17,  18, 138],
		[ 101,  38,  60, 138,  55,  70,  43,  26, 142],
		[ 138,  45,  61,  62, 219,   1,  81, 188,  64],
		[  32,  41,  20, 117, 151, 142,  20,  21, 163],
		[ 112,  19,  12,  61, 195, 128,  48,   4,  24]
	]
];

//Section 11.4 Tree for determining macroblock the chroma mode
static KEYFRAME_UV_MODE_TREE: [i8, ..6] = [
	-DC_PRED, 2,
	-V_PRED, 4,
	-H_PRED, -TM_PRED
];

//Probabilities for determining macroblock mode
static KEYFRAME_UV_MODE_PROBS: [Prob, ..3] = [142, 114, 183];

//Section 13.4
type TokenProbTables = [[[[Prob, ..NUM_DCT_TOKENS - 1], ..3], ..8], ..4];

//Probabilities that a token's probability will be updated
static COEFF_UPDATE_PROBS: TokenProbTables = [
        [
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [176, 246, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [223, 241, 252, 255, 255, 255, 255, 255, 255, 255, 255],
                        [249, 253, 253, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 244, 252, 255, 255, 255, 255, 255, 255, 255, 255],
                        [234, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [253, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 246, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [239, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 248, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [251, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [251, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 254, 253, 255, 254, 255, 255, 255, 255, 255, 255],
                        [250, 255, 254, 255, 254, 255, 255, 255, 255, 255, 255],
                        [254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
        ],
        [
                [
                        [217, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [225, 252, 241, 253, 255, 255, 254, 255, 255, 255, 255],
                        [234, 250, 241, 250, 253, 255, 253, 254, 255, 255, 255],
                ],
                [
                        [255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [223, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [238, 253, 254, 254, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 248, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [249, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 253, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [247, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [252, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [253, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 254, 253, 255, 255, 255, 255, 255, 255, 255, 255],
                        [250, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
        ],
        [
                [
                        [186, 251, 250, 255, 255, 255, 255, 255, 255, 255, 255],
                        [234, 251, 244, 254, 255, 255, 255, 255, 255, 255, 255],
                        [251, 251, 243, 253, 254, 255, 254, 255, 255, 255, 255],
                ],
                [
                        [255, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [236, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [251, 253, 253, 254, 254, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
        ],
        [
                [
                        [248, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [250, 254, 252, 254, 255, 255, 255, 255, 255, 255, 255],
                        [248, 254, 249, 253, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 253, 253, 255, 255, 255, 255, 255, 255, 255, 255],
                        [246, 253, 253, 255, 255, 255, 255, 255, 255, 255, 255],
                        [252, 254, 251, 254, 254, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 254, 252, 255, 255, 255, 255, 255, 255, 255, 255],
                        [248, 254, 253, 255, 255, 255, 255, 255, 255, 255, 255],
                        [253, 255, 254, 254, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 251, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [245, 251, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [253, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 251, 253, 255, 255, 255, 255, 255, 255, 255, 255],
                        [252, 253, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 252, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [249, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 253, 255, 255, 255, 255, 255, 255, 255, 255],
                        [250, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
                [
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                        [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255],
                ],
        ]
];

//Section 13.5
//Default Probabilities for tokens
static COEFF_PROBS: TokenProbTables = [
	[
        	[
                        [128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128],
                        [128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128],
                        [128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128],
                ],
                [
                        [253, 136, 254, 255, 228, 219, 128, 128, 128, 128, 128],
                        [189, 129, 242, 255, 227, 213, 255, 219, 128, 128, 128],
                        [106, 126, 227, 252, 214, 209, 255, 255, 128, 128, 128],
                ],
                [
                        [1, 98, 248, 255, 236, 226, 255, 255, 128, 128, 128],
                        [181, 133, 238, 254, 221, 234, 255, 154, 128, 128, 128],
                        [78, 134, 202, 247, 198, 180, 255, 219, 128, 128, 128],
                ],
                [
                        [1, 185, 249, 255, 243, 255, 128, 128, 128, 128, 128],
                        [184, 150, 247, 255, 236, 224, 128, 128, 128, 128, 128],
                        [77, 110, 216, 255, 236, 230, 128, 128, 128, 128, 128],
                ],
                [
                        [1, 101, 251, 255, 241, 255, 128, 128, 128, 128, 128],
                        [170, 139, 241, 252, 236, 209, 255, 255, 128, 128, 128],
                        [37, 116, 196, 243, 228, 255, 255, 255, 128, 128, 128],
                ],
                [
                        [1, 204, 254, 255, 245, 255, 128, 128, 128, 128, 128],
                        [207, 160, 250, 255, 238, 128, 128, 128, 128, 128, 128],
                        [102, 103, 231, 255, 211, 171, 128, 128, 128, 128, 128],
                ],
                [
                        [1, 152, 252, 255, 240, 255, 128, 128, 128, 128, 128],
                        [177, 135, 243, 255, 234, 225, 128, 128, 128, 128, 128],
                        [80, 129, 211, 255, 194, 224, 128, 128, 128, 128, 128],
                ],
                [
                        [1, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                        [246, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                        [255, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128],
                ],
        ],
        [
                [
                        [198, 35, 237, 223, 193, 187, 162, 160, 145, 155, 62],
                        [131, 45, 198, 221, 172, 176, 220, 157, 252, 221, 1],
                        [68, 47, 146, 208, 149, 167, 221, 162, 255, 223, 128],
                ],
                [
                        [1, 149, 241, 255, 221, 224, 255, 255, 128, 128, 128],
                        [184, 141, 234, 253, 222, 220, 255, 199, 128, 128, 128],
                        [81, 99, 181, 242, 176, 190, 249, 202, 255, 255, 128],
                ],
                [
                        [1, 129, 232, 253, 214, 197, 242, 196, 255, 255, 128],
                        [99, 121, 210, 250, 201, 198, 255, 202, 128, 128, 128],
                        [23, 91, 163, 242, 170, 187, 247, 210, 255, 255, 128],
                ],
                [
                        [1, 200, 246, 255, 234, 255, 128, 128, 128, 128, 128],
                        [109, 178, 241, 255, 231, 245, 255, 255, 128, 128, 128],
                        [44, 130, 201, 253, 205, 192, 255, 255, 128, 128, 128],
                ],
                [
                        [1, 132, 239, 251, 219, 209, 255, 165, 128, 128, 128],
                        [94, 136, 225, 251, 218, 190, 255, 255, 128, 128, 128],
                        [22, 100, 174, 245, 186, 161, 255, 199, 128, 128, 128],
                ],
                [
                        [1, 182, 249, 255, 232, 235, 128, 128, 128, 128, 128],
                        [124, 143, 241, 255, 227, 234, 128, 128, 128, 128, 128],
                        [35, 77, 181, 251, 193, 211, 255, 205, 128, 128, 128],
                ],
                [
                        [1, 157, 247, 255, 236, 231, 255, 255, 128, 128, 128],
                        [121, 141, 235, 255, 225, 227, 255, 255, 128, 128, 128],
                        [45, 99, 188, 251, 195, 217, 255, 224, 128, 128, 128],
                ],
                [
                        [1, 1, 251, 255, 213, 255, 128, 128, 128, 128, 128],
                        [203, 1, 248, 255, 255, 128, 128, 128, 128, 128, 128],
                        [137, 1, 177, 255, 224, 255, 128, 128, 128, 128, 128],
                ],
        ],
        [
                [
                        [253, 9, 248, 251, 207, 208, 255, 192, 128, 128, 128],
                        [175, 13, 224, 243, 193, 185, 249, 198, 255, 255, 128],
                        [73, 17, 171, 221, 161, 179, 236, 167, 255, 234, 128],
                ],
                [
                        [1, 95, 247, 253, 212, 183, 255, 255, 128, 128, 128],
                        [239, 90, 244, 250, 211, 209, 255, 255, 128, 128, 128],
                        [155, 77, 195, 248, 188, 195, 255, 255, 128, 128, 128],
                ],
                [
                        [1, 24, 239, 251, 218, 219, 255, 205, 128, 128, 128],
                        [201, 51, 219, 255, 196, 186, 128, 128, 128, 128, 128],
                        [69, 46, 190, 239, 201, 218, 255, 228, 128, 128, 128],
                ],
                [
                        [1, 191, 251, 255, 255, 128, 128, 128, 128, 128, 128],
                        [223, 165, 249, 255, 213, 255, 128, 128, 128, 128, 128],
                        [141, 124, 248, 255, 255, 128, 128, 128, 128, 128, 128],
                ],
                [
                        [1, 16, 248, 255, 255, 128, 128, 128, 128, 128, 128],
                        [190, 36, 230, 255, 236, 255, 128, 128, 128, 128, 128],
                        [149, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                ],
                [
                        [1, 226, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                        [247, 192, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                        [240, 128, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                ],
                [
                        [1, 134, 252, 255, 255, 128, 128, 128, 128, 128, 128],
                        [213, 62, 250, 255, 255, 128, 128, 128, 128, 128, 128],
                        [55, 93, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                ],
                [
                        [128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128],
                        [128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128],
                        [128, 128, 128, 128, 128, 128, 128, 128, 128, 128, 128],
                ],
        ],
        [
                [
                        [202, 24, 213, 235, 186, 191, 220, 160, 240, 175, 255],
                        [126, 38, 182, 232, 169, 184, 228, 174, 255, 187, 128],
                        [61, 46, 138, 219, 151, 178, 240, 170, 255, 216, 128],
                ],
                [
                        [1, 112, 230, 250, 199, 191, 247, 159, 255, 255, 128],
                        [166, 109, 228, 252, 211, 215, 255, 174, 128, 128, 128],
                        [39, 77, 162, 232, 172, 180, 245, 178, 255, 255, 128],
                ],
                [
                        [1, 52, 220, 246, 198, 199, 249, 220, 255, 255, 128],
                        [124, 74, 191, 243, 183, 193, 250, 221, 255, 255, 128],
                        [24, 71, 130, 219, 154, 170, 243, 182, 255, 255, 128],
                ],
                [
                        [1, 182, 225, 249, 219, 240, 255, 224, 128, 128, 128],
                        [149, 150, 226, 252, 216, 205, 255, 171, 128, 128, 128],
                        [28, 108, 170, 242, 183, 194, 254, 223, 255, 255, 128],
                ],
                [
                        [1, 81, 230, 252, 204, 203, 255, 192, 128, 128, 128],
                        [123, 102, 209, 247, 188, 196, 255, 233, 128, 128, 128],
                        [20, 95, 153, 243, 164, 173, 255, 203, 128, 128, 128],
                ],
                [
                        [1, 222, 248, 255, 216, 213, 128, 128, 128, 128, 128],
                        [168, 175, 246, 252, 235, 205, 255, 255, 128, 128, 128],
                        [47, 116, 215, 255, 211, 212, 255, 255, 128, 128, 128],
                ],
                [
                        [1, 121, 236, 253, 212, 214, 255, 255, 128, 128, 128],
                        [141, 84, 213, 252, 201, 202, 255, 219, 128, 128, 128],
                        [42, 80, 160, 240, 162, 185, 255, 205, 128, 128, 128],
                ],
                [
                        [1, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                        [244, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                        [238, 1, 255, 128, 128, 128, 128, 128, 128, 128, 128],
                ]
        ]
];

struct BoolReader<R> {
	pub r: R,

	range: u32,
	value: u32,
	bit_count: u8
}

impl<R: Reader> BoolReader<R> {
	pub fn new(r: R) -> BoolReader<R> {
		BoolReader {r: r, range: 0, value: 0, bit_count: 0}
	}

	pub fn init(&mut self) -> IoResult<()> {
		let b = try!(self.r.read_be_u16());

		self.value = b as u32;
		self.range = 255;
		self.bit_count = 0;

		Ok(())
	}

	pub fn read_bool(&mut self, probability: u8) -> IoResult<u8> {
		let split = 1 + (((self.range - 1) * probability as u32) >> 8);
		let bigsplit = split << 8;

		let retval = if self.value >= bigsplit {
			self.range -= split;
			self.value -= bigsplit;
			1
		} else {
			self.range = split;
			0
		};

		while self.range < 128 {
			self.value <<= 1;
			self.range <<= 1;
			self.bit_count += 1;

			if self.bit_count == 8 {
				self.bit_count = 0;

				let b = try!(self.r.read_u8());
				self.value |= b as u32;
			}
		}

		Ok(retval)
	}

	pub fn read_literal(&mut self, n: u8) -> IoResult<u8> {
		let mut v = 0;
		let mut n = n;

		while n != 0{
			v = (v << 1) + try!(self.read_bool(128));

			n -= 1;
		}

		Ok(v)
	}

	pub fn read_magnitude_and_sign(&mut self, n: u8) -> IoResult<i32> {
		let magnitude = try!(self.read_literal(n));
		let sign = try!(self.read_literal(1));

		let v = if sign == 1 {-1 * magnitude as i32}
			else {magnitude as i32};

		Ok(v)
	}

	pub fn read_with_tree(&mut self, tree: &[i8], probs: &[Prob]) -> IoResult<i8> {
		let mut index = 0;

		loop {
			let v = try!(self.read_bool(probs[index >> 1]));
			let v = index + v as i8;
			index = tree[v as uint];

			if index <= 0 {
				break
			}
		}

		Ok(-index)
	}

	pub fn read_flag(&mut self) -> IoResult<bool> {
		Ok(0 != try!(self.read_literal(1)))
	}
}

struct MacroBlock {
	bpred: [i8, ..16],
	luma_mode: i8,
        chroma_mode: i8,
}

#[deriving(Default)]
struct Frame {
	keyframe: bool,
	version: u8,
	for_display: bool,

	//Section 9.2
	pixel_type: u8,

	//Section 9.4 and 15
	filter: u8,
	filter_level: u8,
	sharpness_level: u8,

	//Section 9.10
	prob_intra: Prob,

	//Section 9.11
	prob_skip_false: Option<Prob>
}

#[deriving(Default)]
struct Segment {
	absolute_values: bool,
	quantizer_level: i8,
	loopfilter_level: i8,
}

struct VP8<R> {
	b: BoolReader<R>,

	width: u16,
	height: u16,

	frame: Frame,

	segments_enabled: bool,
	segments_update_map: bool,
	segment_feature_mode: bool,
	segment: [Segment, ..MAX_SEGMENTS],
	segment_tree_probs: [Prob, ..3],

	token_probs: ~TokenProbTables,

	top_macroblocks: ~[MacroBlock],
	left_macroblock: MacroBlock,
}

impl<R: Reader> VP8<R> {
	pub fn new(r: R) -> VP8<R> {
		let f: Frame = Default::default();
		let s: Segment = Default::default();

		VP8 {
			b: BoolReader::new(r),

			width: 0,
			height: 0,

			frame: f,
			segments_enabled: false,
			segments_update_map: false,
			segment_feature_mode: false,
			segment: [s, ..MAX_SEGMENTS],

			segment_tree_probs: [255u8, ..3],
			token_probs: ~COEFF_PROBS,

			top_macroblocks: ~[],
			left_macroblock: MacroBlock {bpred: [0i8, ..16], luma_mode: 0},
		}
	}

	fn update_token_probabilities(&mut self) -> IoResult<()> {
		for i in range(0, 4) {
			for j in range(0, 8) {
				for k in range(0, 3) {
					for t in range(0, NUM_DCT_TOKENS - 1) {
						let prob = COEFF_UPDATE_PROBS[i][j][k][t];
						let update = try!(self.b.read_bool(prob));
						if update != 0 {
							let v = try!(self.b.read_literal(8));
							self.token_probs[i][j][k][t] = v as u8;
						}
					}
				}
			}
		}

		Ok(())
	}

	fn read_quantization_indices(&mut self) -> IoResult<()> {
		let y_ac_qindex = try!(self.b.read_literal(7));
		println!("Y AC index: {}", y_ac_qindex);

		let y_dc_delta_present = try!(self.b.read_flag());
		let y_dc_qindex_delta = if y_dc_delta_present {
			try!(self.b.read_magnitude_and_sign(4))
		} else {
			0
		};
		println!("Y DC index delta: {}", y_dc_qindex_delta);

		let y2_dc_delta_present = try!(self.b.read_flag());
		let y2_dc_qindex_delta = if y2_dc_delta_present {
			try!(self.b.read_magnitude_and_sign(4))
		} else {
			0
		};

		println!("Y2 DC index delta: {}", y2_dc_qindex_delta);

		let y2_ac_delta_present = try!(self.b.read_flag());
		let y2_ac_qindex_delta = if y2_ac_delta_present {
			try!(self.b.read_magnitude_and_sign(4))
		} else {
			0
		};

		println!("Y2 AC index delta: {}", y2_ac_qindex_delta);

		let uv_dc_delta_present = try!(self.b.read_flag());
		let uv_dc_qindex_delta = if uv_dc_delta_present {
			try!(self.b.read_magnitude_and_sign(4))
		} else {
			0
		};
		println!("Chroma DC index delta: {}", uv_dc_qindex_delta);

		let uv_ac_delta_present = try!(self.b.read_flag());
		let uv_ac_qindex_delta = if uv_ac_delta_present {
			try!(self.b.read_magnitude_and_sign(4))
		} else {
			0
		};

		println!("Chroma AC index delta: {}", uv_ac_qindex_delta);

		Ok(())
	}

	fn read_loop_filter_adjustments(&mut self) -> IoResult<()> {
		let mode_ref_lf_delta_update = try!(self.b.read_flag());
		if mode_ref_lf_delta_update {
			for i in range(0, 4) {
				let ref_frame_delta_update_flag = try!(self.b.read_flag());

				let delta = if ref_frame_delta_update_flag {
					try!(self.b.read_magnitude_and_sign(6))
				} else {
					0
				};

				println!("\tref delta update {0} - {1}", i, delta);
			}

			for i in range(0, 4) {
				let mb_mode_delta_update_flag = try!(self.b.read_flag());

				let delta = if mb_mode_delta_update_flag {
					try!(self.b.read_magnitude_and_sign(6))
				} else {
					0
				};

				println!("\tmode delta update {0} - {1}", i, delta);
			}
		}

		Ok(())
	}

	fn read_segment_updates(&mut self) -> IoResult<()> {
		//Section 9.3
		self.segments_update_map = try!(self.b.read_flag());
		let update_segment_feature_data = try!(self.b.read_flag());

		if update_segment_feature_data {
			self.segment_feature_mode = try!(self.b.read_flag());

			for i in range(0, MAX_SEGMENTS) {
				let update = try!(self.b.read_flag());

				self.segment[i].quantizer_level = if update {
					try!(self.b.read_magnitude_and_sign(7))
				} else {
					0
				} as i8;
			}

			for i in range(0, MAX_SEGMENTS) {
				let update = try!(self.b.read_flag());

				self.segment[i].loopfilter_level = if update {
					try!(self.b.read_magnitude_and_sign(6))
				} else {
					0
				} as i8;
			}
		}

		if self.segments_update_map {
			for i in range(0, 3) {
				let update = try!(self.b.read_flag());

		 		self.segment_tree_probs[i] = if update {
					try!(self.b.read_literal(8))
				} else {
					255
				} as u8;
			}
		}

		Ok(())
	}

	fn read_frame_header(&mut self) -> IoResult<()> {
		let mut tag = [0u8, ..3];
		let _ = try!(self.b.r.read(tag));

		self.frame.keyframe = tag[0] & 1 == 0;
		self.frame.version = (tag[0] >> 1) & 7;
		self.frame.for_display = (tag[0] >> 4) & 1 != 0;

		let first_partition_size = ((tag[2] as u32 << 16) | (tag[1] as u32 << 8) | tag[0] as u32) >> 5;
		println!("first partition size {}", first_partition_size);

		if self.frame.keyframe {
			let _ = try!(self.b.r.read(tag));
			assert!(tag == [0x9d, 0x01, 0x2a]);

			let w = try!(self.b.r.read_le_u16());
			let h = try!(self.b.r.read_le_u16());

			self.width = w & 0x3FFF;
			self.height = h & 0x3FFF;

			println!("width {0} height {1}", self.width, self.height);
			self.top_macroblocks = init_top_macroblocks(self.width as uint);
			self.left_macroblock = MacroBlock{..self.top_macroblocks[0]};
		}

		//initialise binary decoder
		let _ = try!(self.b.init());

		if self.frame.keyframe {
			let color_space = try!(self.b.read_literal(1));
			self.frame.pixel_type = try!(self.b.read_literal(1));
			assert!(color_space == 0);
		}

		self.segments_enabled = try!(self.b.read_flag());
		if self.segments_enabled {
			let _ = try!(self.read_segment_updates());
		}

		self.frame.filter          = try!(self.b.read_literal(1));
		self.frame.filter_level    = try!(self.b.read_literal(6));
		self.frame.sharpness_level = try!(self.b.read_literal(3));

		let lf_adjust_enable = try!(self.b.read_flag());
		if lf_adjust_enable {
			let _ = try!(self.read_loop_filter_adjustments());
		}

		let num_partitions = 1 << try!(self.b.read_literal(2));
		if num_partitions > 1 {
			fail!("read partition sizes");
		}

		let _ = try!(self.read_quantization_indices());

		if !self.frame.keyframe {
			//9.7 refresh golden frame and altref frame
			fail!("unimplemented")
		} else {
			//Refresh entropy probs ?????
			let _ = try!(self.b.read_literal(1));
		}

		let _ = try!(self.update_token_probabilities());

		let mb_no_skip_coeff = try!(self.b.read_literal(1));
		self.frame.prob_skip_false = if mb_no_skip_coeff == 1 {
			Some(try!(self.b.read_literal(8)))
		} else {
			None
		};

		if !self.frame.keyframe {
			//9.10 remaining frame data
			fail!("unimplemented")
		} else {
			//TODO Reset motion vectors
		}

		Ok(())
	}

	pub fn read_macroblock_header(&mut self) -> IoResult<(i8,> {
		if self.segments_enabled && self.segments_update_map {
			let segment_id = try!(self.b.read_with_tree(SEGMENT_ID_TREE,
								    self.segment_tree_probs));
			println!("segment id: {}", segment_id);
		}

		let skip_coeff = if self.frame.prob_skip_false.is_some() {
			1 == try!(self.b.read_bool(*self.frame.prob_skip_false
							      .get_ref()))
		} else {
			false
		};

		if skip_coeff {
			println!("macro block has no non-zero coeffs");
		}

		let inter_predicted = if !self.frame.keyframe {
			1 == try!(self.b.read_bool(self.frame.prob_intra))
		} else {
			false
		};

		if inter_predicted {
			fail!("inter prediction not implemented");
		}

		if self.frame.keyframe {
			let mbx = 1;
			//intra preditcion
			let ymode = try!(self.b.read_with_tree(KEYFRAME_YMODE_TREE,
							       KEYFRAME_YMODE_PROBS));
			println!("ymode: {}", ymode);

			if ymode == B_PRED {
				for y in range(0, 4) {
					for x in range(0, 4) {
						let top  = self.top_macroblocks[mbx - 1].bpred[x];
						let left = self.left_macroblock.bpred[y];
						let bmode = try!(self.b.read_with_tree(KEYFRAME_BPRED_MODE_TREE,
										       KEYFRAME_BPRED_MODE_PROBS[top][left]));
						println!("bmode: {}", bmode);
						self.top_macroblocks[mbx - 1].bpred[x] = bmode;
						self.left_macroblock.bpred[y * 4 + x] = bmode;
					}
				}
			}

			let uvmode = try!(self.b.read_with_tree(KEYFRAME_UV_MODE_TREE,
								KEYFRAME_UV_MODE_PROBS));
			println!("uvmode: {}", uvmode);
		}

		Ok(())
	}

	pub fn decode(&mut self) -> IoResult<()> {
		let _ = try!(self.read_frame_header());

		self.read_macroblock_header()
	}
}

fn prepare_intra_predict() {

}

fn init_top_macroblocks(width: uint) -> ~[MacroBlock] {
	let mb_width = (width + 15) / 16;

	let mb = MacroBlock {
		//Section 11.3 #3
		bpred: [B_DC_PRED, ..16],
		luma_mode: DC_PRED
	};

	slice::from_fn(mb_width + 2, |_| mb)
}

fn predict_VPRED(above: &[u8], size: uint, dest: &mut [u8]) {
        for y in range(0, size) {
                for x in range(0, size) {
                        dest[x + size * y] = above[x];
                }
        }
}

fn predict_HPRED(left: &[u8], size: uint, dest: &mut [u8]) {
        for y in range(0, size) {
                for x in range(0, size) {
                        dest[x + size * y] = left[y];
                }
        }
}

fn predict_DCPRED(left: Option<&[u8]>, above: Option<&[u8]>, size: uint, dest: &mut [u8]) {
        let mut sum = 0;
        let mut shf = if size == 8 {2} else {3};;

        if left.is_some() {
                sum += left.get_ref().iter().sum();
                shf += 1;
        }

        if above.is_some() {
                sum += above.get_ref().iter().sum();
                shf += 1;
        }

        let dcval = if left.is_none() && above.is_none() {
                128u16
        } else {
                (sum + (1 << (shf - 1))) >> shf
        }

        for i in range(0, size * size) {
                dest[i] = dcval as u8;
        }
}

fn predict_TMPRED(p: u8, left: &[u8], above: &[u8], size: uint, dest: &mut [u8]) {
        for y in range(0, size) {
                for x in range(0, size) {
                        let pred = left[y] as i16 + above[x] as i16 - p as i16;
                        dest[x + size * y] = clip(pred);
                }
        }
}

fn predict_BDCPRED(left: &[u8], above: &[u8], dest: &mut [u8]) {
        let mut v = 0;
        for i in range(0, 4) {
                v += left[i] as u16 + above[i] as u16;
        }

        v >>= 3;

        for y in range(0, 4) {
                for x in range(0, 4) {
                        dest[x + y * 4] = v as u8;
                }
        }
}

fn predict_BVEPRED(above: &[u8], dest: &mut [u8]) {
        for i in range(0, 4) {
                let a = avg3p(above, i);

                dest[0 + 4 * i] = a;
                dest[1 + 4 * i] = a;
                dest[2 + 4 * i] = a;
                dest[3 + 4 * i] = a;
        }
}

fn predict_BHEPRED(left: &[u8], dest: &mut [u8]) {
        for i in range(0, 4) {
                let a = if i < 3 { avg3p(left, i) }
                        else { avg3(left[3], left[4], left[4]) };

                dest[i + 4 * 1] = a;
                dest[i + 4 * 2] = a;
                dest[i + 4 * 3] = a;
                dest[i + 4 * 4] = a;
        }
}

fn predict_BLDPRED(above: &[u8], dest: &mut [u8]) {
        dest[0 * 4 + 0] = avg3p(above, 1);
        dest[0 * 4 + 1] = dest[1 * 4 + 0] = avg3p(above, 2);
        dest[0 * 4 + 2] = dest[1 * 4 + 1] = dest[2 * 4 + 0] = avg3p(above, 3);
        dest[0 * 4 + 3] = dest[1 * 4 + 2] = dest[2 * 4 + 1] = dest[3 * 4 + 0] = avg3p(above, 4);
        dest[1 * 4 + 3] = dest[2 * 4 + 2] = dest[3 * 4 + 1] = avg3p(above, 5);
        dest[2 * 4 + 3] = dest[3 * 4 + 2] = avg3p(above, 6);
        dest[3 * 4 + 3] = avg3(above[6], above[7], above[7]);
}

fn predict_BRDPRED(edge: &[u8], dest: &mut [u8]) {
        dest[3 * 4 + 0] = avg3p(edge, 1);
        dest[3 * 4 + 1] = dest[2 * 4 + 0] = avg3p(edge, 2);
        dest[3 * 4 + 2] = dest[2 * 4 + 1] = dest[1 * 4 + 0] = avg3p(edge, 3);
        dest[3 * 4 + 3] = dest[2 * 4 + 2] = dest[1 * 4 + 1] = dest[0 * 4 + 0] = avg3p(edge, 4);
        dest[2 * 4 + 3] = dest[1 * 4 + 2] = dest[0 * 4 + 1] = avg3p(edge, 5);
        dest[1 * 4 + 3] = dest[0 * 4 + 2] = avg3p(edge, 6);
        dest[0 * 4 + 3] = avg3p(edge, 7);
}

fn predict_BVRPRED(edge: &[u8], dest: &mut [u8]) {
        dest[3 * 4 + 0] = avg3p(edge, 2);
        dest[2 * 4 + 0] = avg3p(edge, 3);
        dest[3 * 4 + 1] = dest[1 * 4 + 0] = avg3p(edge, 4);
        dest[2 * 4 + 1] = dest[0 * 4 + 0] = avg2p(edge, 4);
        dest[3 * 4 + 2] = dest[1 * 4 + 1] = avg3p(edge, 5);
        dest[2 * 4 + 2] = dest[0 * 4 + 1] = avg2p(edge, 5);
        dest[3 * 4 + 3] = dest[1 * 4 + 2] = avg3p(edge, 6);
        dest[2 * 4 + 3] = dest[0 * 4 + 2] = avg2p(edge, 6);
        dest[1 * 4 + 3] = avg3p(edge, 7);
        dest[0 * 4 + 3] = avg2p(edge, 7);
}

fn predict_BVLPRED(above: &[u8], dest: &mut [u8]) {
        dest[0 * 4 + 0] = avg2p(above, 0);
        dest[1 * 4 + 0] = avg3p(above, 1);
        dest[2 * 4 + 0] = dest[0 * 4 + 1] = avg2p(above, 1);
        dest[1 * 4 + 1] = dest[3 * 4 + 0] = avg3p(above, 2);
        dest[2 * 4 + 1] = dest[0 * 4 + 2] = avg2p(above, 2);
        dest[3 * 4 + 1] = dest[1 * 4 + 2] = avg3p(above, 3);
        dest[2 * 4 + 2] = dest[0 * 4 + 3] = avg2p(above, 3);
        dest[3 * 4 + 2] = dest[1 * 4 + 3] = avg3p(above, 4);
        dest[2 * 4 + 3] = avg3p(above, 5);
        dest[3 * 4 + 3] = avg3p(above, 6);
}

fn predict_BHDPRED(edge: &[u8], dest: &mut [u8]) {
        edge[3 * 4 + 0] = avg2p(edge, 0);
        edge[3 * 4 + 1] = avg3p(edge, 1);
        edge[2 * 4 + 0] = edge[3 * 4 + 2] = avg2p(edge, 1);
        edge[2 * 4 + 1] = edge[3 * 4 + 3] = avg3p(edge, 2);
        edge[2 * 4 + 2] = edge[1 * 4 + 0] = avg2p(edge, 2);
        edge[2 * 4 + 3] = edge[1 * 4 + 1] = avg3p(edge, 3);
        edge[1 * 4 + 2] = edge[0 * 4 + 0] = avg2p(edge, 3);
        edge[1 * 4 + 3] = edge[0 * 4 + 1] = avg3p(edge, 4);
        edge[0 * 4 + 2] = avg3p(edge, 5);
        edge[0 * 4 + 3] = avg3p(edge, 6);
}

fn predict_BHUPRED(left: &[u8], dest: &mut [u8]) {
        dest[0 * 4 + 0] = avg2p(left, 0);
        dest[0 * 4 + 1] = avg3p(left, 1);
        dest[0 * 4 + 2] = dest[1 * 4 + 0] = avg2p(left, 1);
        dest[0 * 4 + 3] = dest[1 * 4 + 1] = avg3p(left, 2);
        dest[1 * 4 + 2] = dest[2 * 4 + 0] = avg2p(left, 2);
        dest[1 * 4 + 3] = dest[2 * 4 + 1] = avg3(left[2], left[3], left[3]);
        dest[2 * 4 + 2] = dest[2 * 4 + 3] = dest[3 * 4 + 0] = dest[3 * 4 + 1] = dest[3 * 4 + 2] = dest[3 * 4 + 3] = left[3];
}

fn clip(a: i16) -> u8 {
        if a < 0 { 0u8 }
        else if a > 255 { 255u8 }
        else { a as u8 }
}