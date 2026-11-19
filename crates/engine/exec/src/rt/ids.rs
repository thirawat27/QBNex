//! Numbers of the builtin names that the runtime's two largest dispatches
//! (`call_main` and `display::call`) match on. The VM looks a builtin's number
//! up once when it loads the program, so those matches compare integers instead
//! of trying the names one after another at every call. Generated from the
//! names in those matches; a name without a number is [`NONE`].

pub(crate) type Id = u16;

/// A name neither dispatch matches by number.
pub(crate) const NONE: Id = 0;
pub(crate) const PRINTS: Id = 1;
pub(crate) const PRINTZONE: Id = 2;
pub(crate) const PRINTNL: Id = 3;
pub(crate) const PRINTTAB: Id = 4;
pub(crate) const PRINTSPC: Id = 5;
pub(crate) const WRITES: Id = 6;
pub(crate) const WRITESEP: Id = 7;
pub(crate) const USINGBEGIN: Id = 8;
pub(crate) const USINGS: Id = 9;
pub(crate) const USINGEND: Id = 10;
pub(crate) const VAL: Id = 11;
pub(crate) const VAL_F80: Id = 12;
pub(crate) const HEX_S: Id = 13;
pub(crate) const OCT_S: Id = 14;
pub(crate) const BIN_S: Id = 15;
pub(crate) const MKI_S: Id = 16;
pub(crate) const MKL_S: Id = 17;
pub(crate) const MKS_S: Id = 18;
pub(crate) const MKD_S: Id = 19;
pub(crate) const CVI: Id = 20;
pub(crate) const CVL: Id = 21;
pub(crate) const CVS: Id = 22;
pub(crate) const CVD: Id = 23;
pub(crate) const MKSMBF_S: Id = 24;
pub(crate) const MKDMBF_S: Id = 25;
pub(crate) const CVSMBF: Id = 26;
pub(crate) const CVDMBF: Id = 27;
pub(crate) const LEN: Id = 28;
pub(crate) const LEFT_S: Id = 29;
pub(crate) const RIGHT_S: Id = 30;
pub(crate) const MID_S: Id = 31;
pub(crate) const MID_S_N: Id = 32;
pub(crate) const MIDSET: Id = 33;
pub(crate) const MIDSET_N: Id = 34;
pub(crate) const CHR_S: Id = 35;
pub(crate) const ASC: Id = 36;
pub(crate) const ASC_N: Id = 37;
pub(crate) const INSTR: Id = 38;
pub(crate) const INSTR_N: Id = 39;
pub(crate) const INSTRREV: Id = 40;
pub(crate) const INSTRREV_N: Id = 41;
pub(crate) const UCASE_S: Id = 42;
pub(crate) const LCASE_S: Id = 43;
pub(crate) const LTRIM_S: Id = 44;
pub(crate) const RTRIM_S: Id = 45;
pub(crate) const TRIM_S: Id = 46;
pub(crate) const SPACE_S: Id = 47;
pub(crate) const STRING_S: Id = 48;
pub(crate) const STRING_S_S: Id = 49;
pub(crate) const LSET: Id = 50;
pub(crate) const RSET: Id = 51;
pub(crate) const ABS_I64: Id = 52;
pub(crate) const ABS_F32: Id = 53;
pub(crate) const ABS_F64: Id = 54;
pub(crate) const ABS_F80: Id = 55;
pub(crate) const SGN_I64: Id = 56;
pub(crate) const SGN_F80: Id = 57;
pub(crate) const SGN_F64: Id = 58;
pub(crate) const INT_F32: Id = 59;
pub(crate) const INT_F64: Id = 60;
pub(crate) const FIX_F32: Id = 61;
pub(crate) const FIX_F64: Id = 62;
pub(crate) const INT_F80: Id = 63;
pub(crate) const FIX_F80: Id = 64;
pub(crate) const SQR: Id = 65;
pub(crate) const SIN: Id = 66;
pub(crate) const COS: Id = 67;
pub(crate) const TAN: Id = 68;
pub(crate) const ATN: Id = 69;
pub(crate) const LOG: Id = 70;
pub(crate) const EXP_S: Id = 71;
pub(crate) const EXP: Id = 72;
pub(crate) const ROUND_F64: Id = 73;
pub(crate) const RND: Id = 74;
pub(crate) const RND_N: Id = 75;
pub(crate) const RANDOMIZE: Id = 76;
pub(crate) const RANDOMIZEUSING: Id = 77;
pub(crate) const TIMER_N: Id = 78;
pub(crate) const TIMER: Id = 79;
pub(crate) const ERR: Id = 80;
pub(crate) const ERL: Id = 81;
pub(crate) const ERRORLINE: Id = 82;
pub(crate) const INCLERRORLINE: Id = 83;
pub(crate) const INCLERRORFILE_S: Id = 84;
pub(crate) const LINEINPUT: Id = 85;
pub(crate) const INPUT_S: Id = 86;
pub(crate) const INPUTPROMPT: Id = 87;
pub(crate) const INPUTPROMPT_T: Id = 88;
pub(crate) const INPUTITEM_S: Id = 89;
pub(crate) const INPUTITEM_N: Id = 90;
pub(crate) const INPUTITEM_I: Id = 91;
pub(crate) const INPUTITEM_X: Id = 92;
pub(crate) const KILL: Id = 93;
pub(crate) const NAME: Id = 94;
pub(crate) const MKDIR: Id = 95;
pub(crate) const RMDIR: Id = 96;
pub(crate) const CHDIR: Id = 97;
pub(crate) const FILEEXISTS: Id = 98;
pub(crate) const DIREXISTS: Id = 99;
pub(crate) const POS: Id = 100;
pub(crate) const FUNC_LPOS: Id = 101;
pub(crate) const CSRLIN: Id = 102;
pub(crate) const SUB_ECHO: Id = 103;
pub(crate) const SUB_STDERR: Id = 104;
pub(crate) const SLEEP: Id = 105;
pub(crate) const ASCSET: Id = 106;
pub(crate) const COMMAND_S: Id = 107;
pub(crate) const COMMAND_S_N: Id = 108;
pub(crate) const ENVIRON_S: Id = 109;
pub(crate) const ENVIRON_S_N: Id = 110;
pub(crate) const ENVIRON: Id = 111;
pub(crate) const SHELL: Id = 112;
pub(crate) const DATE_S: Id = 113;
pub(crate) const TIME_S: Id = 114;
pub(crate) const TIME_S_SET: Id = 115;
pub(crate) const DATE_S_SET: Id = 116;
pub(crate) const DEFLATE_S: Id = 117;
pub(crate) const INFLATE_S: Id = 118;
pub(crate) const INFLATE_S_N: Id = 119;
pub(crate) const VARSEG: Id = 120;
pub(crate) const DEFSEG: Id = 121;
pub(crate) const DEFSEG_D: Id = 122;
pub(crate) const PEEK: Id = 123;
pub(crate) const POKE: Id = 124;
pub(crate) const STRCMP: Id = 125;
pub(crate) const STRICMP: Id = 126;
pub(crate) const CWD_S: Id = 127;
pub(crate) const STARTDIR_S: Id = 128;
pub(crate) const QBG_SCREEN: Id = 129;
pub(crate) const SUB_PSET: Id = 130;
pub(crate) const SUB_PRESET: Id = 131;
pub(crate) const SUB_LINE: Id = 132;
pub(crate) const SUB_CIRCLE: Id = 133;
pub(crate) const SUB_PAINT: Id = 134;
pub(crate) const SUB_PAINT_S: Id = 135;
pub(crate) const QBG_PALETTE: Id = 136;
pub(crate) const QBG_SUB_COLOR: Id = 137;
pub(crate) const QBG_SUB_VIEW: Id = 138;
pub(crate) const QBG_SUB_VIEW_PRINT: Id = 139;
pub(crate) const QBG_SUB_WINDOW: Id = 140;
pub(crate) const FUNC_PMAP: Id = 141;
pub(crate) const FUNC_POINT: Id = 142;
pub(crate) const SUB_PCOPY: Id = 143;
pub(crate) const SUB_DRAW: Id = 144;
pub(crate) const SUB_CLS: Id = 145;
pub(crate) const QBG_SUB_LOCATE: Id = 146;
pub(crate) const SUB_DEST: Id = 147;
pub(crate) const SUB_SOURCE: Id = 148;
pub(crate) const FUNC_DEST: Id = 149;
pub(crate) const FUNC_SOURCE: Id = 150;
pub(crate) const FUNC_DISPLAY: Id = 151;
pub(crate) const FUNC_CONSOLE: Id = 152;
pub(crate) const FUNC_NEWIMAGE: Id = 153;
pub(crate) const SUB_FREEIMAGE: Id = 154;
pub(crate) const SUB_PRINTIMAGE: Id = 155;
pub(crate) const FUNC_COPYIMAGE: Id = 156;
pub(crate) const SUB_DISPLAY: Id = 157;
pub(crate) const FUNC_WIDTH: Id = 158;
pub(crate) const FUNC_HEIGHT: Id = 159;
pub(crate) const FUNC_PIXELSIZE: Id = 160;
pub(crate) const FUNC_RGB: Id = 161;
pub(crate) const FUNC_RGBA: Id = 162;
pub(crate) const FUNC_RED: Id = 163;
pub(crate) const FUNC_GREEN: Id = 164;
pub(crate) const FUNC_BLUE: Id = 165;
pub(crate) const FUNC_ALPHA: Id = 166;
pub(crate) const FUNC_RGB32: Id = 167;
pub(crate) const FUNC_RGBA32: Id = 168;
pub(crate) const FUNC_RED32: Id = 169;
pub(crate) const FUNC_GREEN32: Id = 170;
pub(crate) const FUNC_BLUE32: Id = 171;
pub(crate) const FUNC_ALPHA32: Id = 172;
pub(crate) const FUNC_PALETTECOLOR: Id = 173;
pub(crate) const SUB_PALETTECOLOR: Id = 174;
pub(crate) const SUB_GRAPHICS_GET: Id = 175;
pub(crate) const SUB_GRAPHICS_PUT: Id = 176;
pub(crate) const SUB_PALETTEUSING: Id = 177;
pub(crate) const SUB_FONT: Id = 178;
pub(crate) const FUNC_FONT: Id = 179;
pub(crate) const FUNC_FONTWIDTH: Id = 180;
pub(crate) const FUNC_FONTHEIGHT: Id = 181;
pub(crate) const SUB_AUTODISPLAY: Id = 182;
pub(crate) const FUNC_AUTODISPLAY: Id = 183;
pub(crate) const SCREENPRINT: Id = 184;
pub(crate) const SCREENNL: Id = 185;
pub(crate) const SCREENZONE: Id = 186;
pub(crate) const SCREENTAB: Id = 187;
pub(crate) const SCREENSPC: Id = 188;
pub(crate) const FUNC_SCREEN: Id = 189;
pub(crate) const SUB_DELAY: Id = 190;
pub(crate) const SUB_TITLE: Id = 191;
pub(crate) const FUNC_CSRLIN: Id = 192;
pub(crate) const FUNC_POS: Id = 193;
pub(crate) const QBSUB_WIDTH: Id = 194;
pub(crate) const SUB_SETALPHA: Id = 195;
pub(crate) const SUB_CLEARCOLOR: Id = 196;
pub(crate) const FUNC_CLEARCOLOR: Id = 197;
pub(crate) const SUB_PUTIMAGE: Id = 198;
pub(crate) const SUB_MAPTRIANGLE: Id = 199;
pub(crate) const FUNC_LOADFONT: Id = 200;
pub(crate) const SUB_FREEFONT: Id = 201;
pub(crate) const FUNC_PRINTWIDTH: Id = 202;
pub(crate) const FUNC_LOADIMAGE: Id = 203;
pub(crate) const FUNC_TITLE: Id = 204;
pub(crate) const SUB_BLEND: Id = 205;
pub(crate) const SUB_DONTBLEND: Id = 206;
pub(crate) const SUB_GLRENDER: Id = 207;
pub(crate) const SUB_DISPLAYORDER: Id = 208;
pub(crate) const SUB_DEPTHBUFFER: Id = 209;
pub(crate) const FUNC_BLEND: Id = 210;
pub(crate) const FUNC_DEFAULTCOLOR: Id = 211;
pub(crate) const FUNC_BACKGROUNDCOLOR: Id = 212;
pub(crate) const FUNC_PRINTMODE: Id = 213;
pub(crate) const SUB_PRINTMODE: Id = 214;
pub(crate) const SUB_COPYPALETTE: Id = 215;
pub(crate) const SUB_BLINK: Id = 216;
pub(crate) const FUNC_BLINK: Id = 217;
pub(crate) const SUB_CONTROLCHR: Id = 218;
pub(crate) const FUNC_CONTROLCHR: Id = 219;
pub(crate) const FUNC_CLIPBOARDIMAGE: Id = 220;
pub(crate) const SUB_CLIPBOARDIMAGE: Id = 221;
pub(crate) const SUB_PRINTSTRING: Id = 222;
pub(crate) const SUB_ICON: Id = 223;
pub(crate) const FUNC_SCREENIMAGE: Id = 224;
pub(crate) const SUB_MOUSEMOVE: Id = 225;

/// The names, by number (index 0 is [`NONE`]).
static NAMES: [&str; 226] = [
    "",
    "PRINTS",
    "PRINTZONE",
    "PRINTNL",
    "PRINTTAB",
    "PRINTSPC",
    "WRITES",
    "WRITESEP",
    "USINGBEGIN",
    "USINGS",
    "USINGEND",
    "VAL",
    "VAL.f80",
    "HEX$",
    "OCT$",
    "BIN$",
    "MKI$",
    "MKL$",
    "MKS$",
    "MKD$",
    "CVI",
    "CVL",
    "CVS",
    "CVD",
    "MKSMBF$",
    "MKDMBF$",
    "CVSMBF",
    "CVDMBF",
    "LEN",
    "LEFT$",
    "RIGHT$",
    "MID$",
    "MID$.n",
    "MIDSET",
    "MIDSET.n",
    "CHR$",
    "ASC",
    "ASC.n",
    "INSTR",
    "INSTR.n",
    "_INSTRREV",
    "_INSTRREV.n",
    "UCASE$",
    "LCASE$",
    "LTRIM$",
    "RTRIM$",
    "_TRIM$",
    "SPACE$",
    "STRING$",
    "STRING$.s",
    "LSET",
    "RSET",
    "ABS.i64",
    "ABS.f32",
    "ABS.f64",
    "ABS.f80",
    "SGN.i64",
    "SGN.f80",
    "SGN.f64",
    "INT.f32",
    "INT.f64",
    "FIX.f32",
    "FIX.f64",
    "INT.f80",
    "FIX.f80",
    "SQR",
    "SIN",
    "COS",
    "TAN",
    "ATN",
    "LOG",
    "EXP.s",
    "EXP",
    "_ROUND.f64",
    "RND",
    "RND.n",
    "RANDOMIZE",
    "RANDOMIZEUSING",
    "TIMER.n",
    "TIMER",
    "ERR",
    "ERL",
    "_ERRORLINE",
    "_INCLERRORLINE",
    "_INCLERRORFILE$",
    "LINEINPUT",
    "INPUT$",
    "INPUTPROMPT",
    "INPUTPROMPT.t",
    "INPUTITEM.s",
    "INPUTITEM.n",
    "INPUTITEM.i",
    "INPUTITEM.x",
    "KILL",
    "NAME",
    "MKDIR",
    "RMDIR",
    "CHDIR",
    "_FILEEXISTS",
    "_DIREXISTS",
    "POS",
    "func_lpos",
    "CSRLIN",
    "sub__echo",
    "sub__stderr",
    "SLEEP",
    "ASCSET",
    "COMMAND$",
    "COMMAND$.n",
    "ENVIRON$",
    "ENVIRON$.n",
    "ENVIRON",
    "SHELL",
    "DATE$",
    "TIME$",
    "TIME$.set",
    "DATE$.set",
    "_DEFLATE$",
    "_INFLATE$",
    "_INFLATE$.n",
    "VARSEG",
    "DEFSEG",
    "DEFSEG.d",
    "PEEK",
    "POKE",
    "_STRCMP",
    "_STRICMP",
    "_CWD$",
    "_STARTDIR$",
    "qbg_screen",
    "sub_pset",
    "sub_preset",
    "sub_line",
    "sub_circle",
    "sub_paint",
    "sub_paint.s",
    "qbg_palette",
    "qbg_sub_color",
    "qbg_sub_view",
    "qbg_sub_view_print",
    "qbg_sub_window",
    "func_pmap",
    "func_point",
    "sub_pcopy",
    "sub_draw",
    "sub_cls",
    "qbg_sub_locate",
    "sub__dest",
    "sub__source",
    "func__dest",
    "func__source",
    "func__display",
    "func__console",
    "func__newimage",
    "sub__freeimage",
    "sub__printimage",
    "func__copyimage",
    "sub__display",
    "func__width",
    "func__height",
    "func__pixelsize",
    "func__rgb",
    "func__rgba",
    "func__red",
    "func__green",
    "func__blue",
    "func__alpha",
    "func__rgb32",
    "func__rgba32",
    "func__red32",
    "func__green32",
    "func__blue32",
    "func__alpha32",
    "func__palettecolor",
    "sub__palettecolor",
    "sub_graphics_get",
    "sub_graphics_put",
    "sub_paletteusing",
    "sub__font",
    "func__font",
    "func__fontwidth",
    "func__fontheight",
    "sub__autodisplay",
    "func__autodisplay",
    "SCREENPRINT",
    "SCREENNL",
    "SCREENZONE",
    "SCREENTAB",
    "SCREENSPC",
    "func_screen",
    "sub__delay",
    "sub__title",
    "func_csrlin",
    "func_pos",
    "qbsub_width",
    "sub__setalpha",
    "sub__clearcolor",
    "func__clearcolor",
    "sub__putimage",
    "sub__maptriangle",
    "func__loadfont",
    "sub__freefont",
    "func__printwidth",
    "func__loadimage",
    "func__title",
    "sub__blend",
    "sub__dontblend",
    "sub__glrender",
    "sub__displayorder",
    "sub__depthbuffer",
    "func__blend",
    "func__defaultcolor",
    "func__backgroundcolor",
    "func__printmode",
    "sub__printmode",
    "sub__copypalette",
    "sub__blink",
    "func__blink",
    "sub__controlchr",
    "func__controlchr",
    "func__clipboardimage",
    "sub__clipboardimage",
    "sub__printstring",
    "sub__icon",
    "func__screenimage",
    "sub__mousemove",
];

/// The constant of each number, as the dispatches spell it (for the tests that
/// check which dispatch has a case for a name).
#[cfg(test)]
pub(crate) static CONSTS: [&str; 226] = [
    "NONE",
    "PRINTS",
    "PRINTZONE",
    "PRINTNL",
    "PRINTTAB",
    "PRINTSPC",
    "WRITES",
    "WRITESEP",
    "USINGBEGIN",
    "USINGS",
    "USINGEND",
    "VAL",
    "VAL_F80",
    "HEX_S",
    "OCT_S",
    "BIN_S",
    "MKI_S",
    "MKL_S",
    "MKS_S",
    "MKD_S",
    "CVI",
    "CVL",
    "CVS",
    "CVD",
    "MKSMBF_S",
    "MKDMBF_S",
    "CVSMBF",
    "CVDMBF",
    "LEN",
    "LEFT_S",
    "RIGHT_S",
    "MID_S",
    "MID_S_N",
    "MIDSET",
    "MIDSET_N",
    "CHR_S",
    "ASC",
    "ASC_N",
    "INSTR",
    "INSTR_N",
    "INSTRREV",
    "INSTRREV_N",
    "UCASE_S",
    "LCASE_S",
    "LTRIM_S",
    "RTRIM_S",
    "TRIM_S",
    "SPACE_S",
    "STRING_S",
    "STRING_S_S",
    "LSET",
    "RSET",
    "ABS_I64",
    "ABS_F32",
    "ABS_F64",
    "ABS_F80",
    "SGN_I64",
    "SGN_F80",
    "SGN_F64",
    "INT_F32",
    "INT_F64",
    "FIX_F32",
    "FIX_F64",
    "INT_F80",
    "FIX_F80",
    "SQR",
    "SIN",
    "COS",
    "TAN",
    "ATN",
    "LOG",
    "EXP_S",
    "EXP",
    "ROUND_F64",
    "RND",
    "RND_N",
    "RANDOMIZE",
    "RANDOMIZEUSING",
    "TIMER_N",
    "TIMER",
    "ERR",
    "ERL",
    "ERRORLINE",
    "INCLERRORLINE",
    "INCLERRORFILE_S",
    "LINEINPUT",
    "INPUT_S",
    "INPUTPROMPT",
    "INPUTPROMPT_T",
    "INPUTITEM_S",
    "INPUTITEM_N",
    "INPUTITEM_I",
    "INPUTITEM_X",
    "KILL",
    "NAME",
    "MKDIR",
    "RMDIR",
    "CHDIR",
    "FILEEXISTS",
    "DIREXISTS",
    "POS",
    "FUNC_LPOS",
    "CSRLIN",
    "SUB_ECHO",
    "SUB_STDERR",
    "SLEEP",
    "ASCSET",
    "COMMAND_S",
    "COMMAND_S_N",
    "ENVIRON_S",
    "ENVIRON_S_N",
    "ENVIRON",
    "SHELL",
    "DATE_S",
    "TIME_S",
    "TIME_S_SET",
    "DATE_S_SET",
    "DEFLATE_S",
    "INFLATE_S",
    "INFLATE_S_N",
    "VARSEG",
    "DEFSEG",
    "DEFSEG_D",
    "PEEK",
    "POKE",
    "STRCMP",
    "STRICMP",
    "CWD_S",
    "STARTDIR_S",
    "QBG_SCREEN",
    "SUB_PSET",
    "SUB_PRESET",
    "SUB_LINE",
    "SUB_CIRCLE",
    "SUB_PAINT",
    "SUB_PAINT_S",
    "QBG_PALETTE",
    "QBG_SUB_COLOR",
    "QBG_SUB_VIEW",
    "QBG_SUB_VIEW_PRINT",
    "QBG_SUB_WINDOW",
    "FUNC_PMAP",
    "FUNC_POINT",
    "SUB_PCOPY",
    "SUB_DRAW",
    "SUB_CLS",
    "QBG_SUB_LOCATE",
    "SUB_DEST",
    "SUB_SOURCE",
    "FUNC_DEST",
    "FUNC_SOURCE",
    "FUNC_DISPLAY",
    "FUNC_CONSOLE",
    "FUNC_NEWIMAGE",
    "SUB_FREEIMAGE",
    "SUB_PRINTIMAGE",
    "FUNC_COPYIMAGE",
    "SUB_DISPLAY",
    "FUNC_WIDTH",
    "FUNC_HEIGHT",
    "FUNC_PIXELSIZE",
    "FUNC_RGB",
    "FUNC_RGBA",
    "FUNC_RED",
    "FUNC_GREEN",
    "FUNC_BLUE",
    "FUNC_ALPHA",
    "FUNC_RGB32",
    "FUNC_RGBA32",
    "FUNC_RED32",
    "FUNC_GREEN32",
    "FUNC_BLUE32",
    "FUNC_ALPHA32",
    "FUNC_PALETTECOLOR",
    "SUB_PALETTECOLOR",
    "SUB_GRAPHICS_GET",
    "SUB_GRAPHICS_PUT",
    "SUB_PALETTEUSING",
    "SUB_FONT",
    "FUNC_FONT",
    "FUNC_FONTWIDTH",
    "FUNC_FONTHEIGHT",
    "SUB_AUTODISPLAY",
    "FUNC_AUTODISPLAY",
    "SCREENPRINT",
    "SCREENNL",
    "SCREENZONE",
    "SCREENTAB",
    "SCREENSPC",
    "FUNC_SCREEN",
    "SUB_DELAY",
    "SUB_TITLE",
    "FUNC_CSRLIN",
    "FUNC_POS",
    "QBSUB_WIDTH",
    "SUB_SETALPHA",
    "SUB_CLEARCOLOR",
    "FUNC_CLEARCOLOR",
    "SUB_PUTIMAGE",
    "SUB_MAPTRIANGLE",
    "FUNC_LOADFONT",
    "SUB_FREEFONT",
    "FUNC_PRINTWIDTH",
    "FUNC_LOADIMAGE",
    "FUNC_TITLE",
    "SUB_BLEND",
    "SUB_DONTBLEND",
    "SUB_GLRENDER",
    "SUB_DISPLAYORDER",
    "SUB_DEPTHBUFFER",
    "FUNC_BLEND",
    "FUNC_DEFAULTCOLOR",
    "FUNC_BACKGROUNDCOLOR",
    "FUNC_PRINTMODE",
    "SUB_PRINTMODE",
    "SUB_COPYPALETTE",
    "SUB_BLINK",
    "FUNC_BLINK",
    "SUB_CONTROLCHR",
    "FUNC_CONTROLCHR",
    "FUNC_CLIPBOARDIMAGE",
    "SUB_CLIPBOARDIMAGE",
    "SUB_PRINTSTRING",
    "SUB_ICON",
    "FUNC_SCREENIMAGE",
    "SUB_MOUSEMOVE",
];

/// The number of builtin `name`, [`NONE`] if it has none.
pub(crate) fn of(name: &str) -> Id {
    static MAP: std::sync::OnceLock<std::collections::HashMap<&'static str, Id>> = std::sync::OnceLock::new();
    let map = MAP.get_or_init(|| NAMES.iter().enumerate().skip(1).map(|(i, n)| (*n, i as Id)).collect());
    map.get(name).copied().unwrap_or(NONE)
}

#[cfg(test)]
mod tests {
    #[test]
    fn numbers_and_names_agree() {
        for (i, n) in super::NAMES.iter().enumerate().skip(1) {
            assert_eq!(super::of(n) as usize, i, "{n}");
        }
        assert_eq!(super::of("no such builtin"), super::NONE);
    }
}
