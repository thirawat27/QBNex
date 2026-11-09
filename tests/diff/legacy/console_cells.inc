// Verbatim from internal/c/libqb.cpp (R12a): code page 437 table, unicode_to_cp437, UTF-8 cell side table.
static uint16 codepage437_to_unicode16[] = {
    0x0020,0x263A,0x263B,0x2665,0x2666,0x2663,0x2660,0x2022,0x25D8,0x25CB,0x25D9,0x2642,0x2640,0x266A,0x266B,0x263C,
    0x25BA,0x25C4,0x2195,0x203C,0x00B6,0x00A7,0x25AC,0x21A8,0x2191,0x2193,0x2192,0x2190,0x221F,0x2194,0x25B2,0x25BC,
    0x0020,0x0021,0x0022,0x0023,0x0024,0x0025,0x0026,0x0027,0x0028,0x0029,0x002A,0x002B,0x002C,0x002D,0x002E,0x002F,
    0x0030,0x0031,0x0032,0x0033,0x0034,0x0035,0x0036,0x0037,0x0038,0x0039,0x003A,0x003B,0x003C,0x003D,0x003E,0x003F,
    0x0040,0x0041,0x0042,0x0043,0x0044,0x0045,0x0046,0x0047,0x0048,0x0049,0x004A,0x004B,0x004C,0x004D,0x004E,0x004F,
    0x0050,0x0051,0x0052,0x0053,0x0054,0x0055,0x0056,0x0057,0x0058,0x0059,0x005A,0x005B,0x005C,0x005D,0x005E,0x005F,
    0x0060,0x0061,0x0062,0x0063,0x0064,0x0065,0x0066,0x0067,0x0068,0x0069,0x006A,0x006B,0x006C,0x006D,0x006E,0x006F,
    0x0070,0x0071,0x0072,0x0073,0x0074,0x0075,0x0076,0x0077,0x0078,0x0079,0x007A,0x007B,0x007C,0x007D,0x007E,0x2302,
    0x00C7,0x00FC,0x00E9,0x00E2,0x00E4,0x00E0,0x00E5,0x00E7,0x00EA,0x00EB,0x00E8,0x00EF,0x00EE,0x00EC,0x00C4,0x00C5,
    0x00C9,0x00E6,0x00C6,0x00F4,0x00F6,0x00F2,0x00FB,0x00F9,0x00FF,0x00D6,0x00DC,0x00A2,0x00A3,0x00A5,0x20A7,0x0192,
    0x00E1,0x00ED,0x00F3,0x00FA,0x00F1,0x00D1,0x00AA,0x00BA,0x00BF,0x2310,0x00AC,0x00BD,0x00BC,0x00A1,0x00AB,0x00BB,
    0x2591,0x2592,0x2593,0x2502,0x2524,0x2561,0x2562,0x2556,0x2555,0x2563,0x2551,0x2557,0x255D,0x255C,0x255B,0x2510,
    0x2514,0x2534,0x252C,0x251C,0x2500,0x253C,0x255E,0x255F,0x255A,0x2554,0x2569,0x2566,0x2560,0x2550,0x256C,0x2567,
    0x2568,0x2564,0x2565,0x2559,0x2558,0x2552,0x2553,0x256B,0x256A,0x2518,0x250C,0x2588,0x2584,0x258C,0x2590,0x2580,
    0x03B1,0x00DF,0x0393,0x03C0,0x03A3,0x03C3,0x00B5,0x03C4,0x03A6,0x0398,0x03A9,0x03B4,0x221E,0x03C6,0x03B5,0x2229,
    0x2261,0x00B1,0x2265,0x2264,0x2320,0x2321,0x00F7,0x2248,0x00B0,0x2219,0x00B7,0x221A,0x207F,0x00B2,0x25A0,0x0020
};
uint32 unicode_to_cp437(uint32 x){
    static int32 i;
    for (i=0;i<=255;i++){
        if (x==codepage437_to_unicode16[i]) return i;
    }
    return 0;
}
static std::vector<std::vector<std::string>> qb_text_utf8_cells(IMG_BUFFERSIZE);
static std::vector<std::vector<uint8>> qb_text_utf8_cell_state(IMG_BUFFERSIZE);
static void qb_release_text_utf8_cells(int32 image_index){
    if (image_index < 0) return;
    if (static_cast<size_t>(image_index) < qb_text_utf8_cells.size()){
        std::vector<std::string>().swap(qb_text_utf8_cells[image_index]);
    }
    if (static_cast<size_t>(image_index) < qb_text_utf8_cell_state.size()){
        std::vector<uint8>().swap(qb_text_utf8_cell_state[image_index]);
    }
}

static int32 qb_img_index(const img_struct *im){
    if (!im || !img) return -1;
    return static_cast<int32>(im - img);
}

static void qb_init_text_utf8_cells(int32 image_index){
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) qb_text_utf8_cells.resize(nimg);
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cell_state.size()) qb_text_utf8_cell_state.resize(nimg);
    if (!img[image_index].text){
        qb_release_text_utf8_cells(image_index);
        return;
    }
    qb_text_utf8_cells[image_index].assign(static_cast<size_t>(img[image_index].width) * img[image_index].height, std::string());
    qb_text_utf8_cell_state[image_index].assign(static_cast<size_t>(img[image_index].width) * img[image_index].height, 0);
}

static void qb_clear_text_utf8_cells_range(int32 image_index,int32 first_cell,int32 cell_count){
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cells.empty() || cell_count <= 0) return;
    if (first_cell < 0) first_cell=0;
    int32 end_cell=first_cell+cell_count;
    if (end_cell > static_cast<int32>(cells.size())) end_cell=static_cast<int32>(cells.size());
    for (int32 i=first_cell;i<end_cell;i++){
        cells[i].clear();
        cell_state[i]=0;
    }
}

static int32 qb_text_page_has_utf8_content(int32 image_index){
    if (image_index < 0 || image_index >= nimg) return 0;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return 0;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    for (size_t i=0;i<cells.size();i++){
        if (!cells[i].empty()) return -1;
    }
    return 0;
}

static void qb_append_utf8_codepoint(std::string &out,uint32 codepoint){
    if (codepoint > 0x10FFFF) codepoint=0xFFFD;
    if (codepoint <= 0x7F){
        out.push_back(static_cast<char>(codepoint));
        return;
    }
    if (codepoint <= 0x7FF){
        out.push_back(static_cast<char>(0xC0 | (codepoint >> 6)));
        out.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
        return;
    }
    if (codepoint <= 0xFFFF){
        out.push_back(static_cast<char>(0xE0 | (codepoint >> 12)));
        out.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
        return;
    }
    out.push_back(static_cast<char>(0xF0 | (codepoint >> 18)));
    out.push_back(static_cast<char>(0x80 | ((codepoint >> 12) & 0x3F)));
    out.push_back(static_cast<char>(0x80 | ((codepoint >> 6) & 0x3F)));
    out.push_back(static_cast<char>(0x80 | (codepoint & 0x3F)));
}

static int32 qb_decode_utf8_codepoint(const uint8 *text,int32 len,uint32 *codepoint){
    if (!text || len <= 0 || !codepoint) return 0;

    const uint8 b0=text[0];
    if (b0 < 0x80){
        *codepoint=b0;
        return 1;
    }

    if ((b0 >= 0xC2) && (b0 <= 0xDF) && len >= 2){
        const uint8 b1=text[1];
        if ((b1 & 0xC0) == 0x80){
            *codepoint=((b0 & 0x1F) << 6) | (b1 & 0x3F);
            return 2;
        }
    }

    if ((b0 >= 0xE0) && (b0 <= 0xEF) && len >= 3){
        const uint8 b1=text[1];
        const uint8 b2=text[2];
        if (((b1 & 0xC0) == 0x80) && ((b2 & 0xC0) == 0x80)){
            uint32 value=((b0 & 0x0F) << 12) | ((b1 & 0x3F) << 6) | (b2 & 0x3F);
            if ((value >= 0x800) && !(value >= 0xD800 && value <= 0xDFFF)){
                if (!((b0 == 0xE0) && (b1 < 0xA0)) && !((b0 == 0xED) && (b1 >= 0xA0))){
                    *codepoint=value;
                    return 3;
                }
            }
        }
    }

    if ((b0 >= 0xF0) && (b0 <= 0xF4) && len >= 4){
        const uint8 b1=text[1];
        const uint8 b2=text[2];
        const uint8 b3=text[3];
        if (((b1 & 0xC0) == 0x80) && ((b2 & 0xC0) == 0x80) && ((b3 & 0xC0) == 0x80)){
            uint32 value=((b0 & 0x07) << 18) | ((b1 & 0x3F) << 12) | ((b2 & 0x3F) << 6) | (b3 & 0x3F);
            if ((value >= 0x10000) && (value <= 0x10FFFF)){
                if (!((b0 == 0xF0) && (b1 < 0x90)) && !((b0 == 0xF4) && (b1 > 0x8F))){
                    *codepoint=value;
                    return 4;
                }
            }
        }
    }

    *codepoint=codepage437_to_unicode16[b0];
    return 1;
}

static int32 qb_is_emoji_like_codepoint(uint32 codepoint){
    if ((codepoint >= 0x1F1E6 && codepoint <= 0x1F1FF) ||
        (codepoint >= 0x1F300 && codepoint <= 0x1FAFF) ||
        (codepoint >= 0x2600 && codepoint <= 0x27BF) ||
        (codepoint >= 0x2300 && codepoint <= 0x23FF) ||
        codepoint == 0x20E3) return -1;
    return 0;
}

static int32 qb_is_zero_width_codepoint(uint32 codepoint){
    if ((codepoint >= 0x0300 && codepoint <= 0x036F) ||
        (codepoint >= 0x1AB0 && codepoint <= 0x1AFF) ||
        (codepoint >= 0x1DC0 && codepoint <= 0x1DFF) ||
        (codepoint >= 0x20D0 && codepoint <= 0x20FF) ||
        (codepoint >= 0xFE20 && codepoint <= 0xFE2F) ||
        (codepoint >= 0xFE00 && codepoint <= 0xFE0F) ||
        (codepoint >= 0xE0100 && codepoint <= 0xE01EF)) return -1;
    if (codepoint == 0x0E31 || codepoint == 0x200C || codepoint == 0x200D || codepoint == 0xFEFF) return -1;
    if ((codepoint >= 0x0E34 && codepoint <= 0x0E3A) || (codepoint >= 0x0E47 && codepoint <= 0x0E4E)) return -1;
    return 0;
}

static int32 qb_is_wide_codepoint(uint32 codepoint){
    if (qb_is_emoji_like_codepoint(codepoint)) return -1;
    if ((codepoint >= 0x1100 && codepoint <= 0x115F) ||
        (codepoint >= 0x2329 && codepoint <= 0x232A) ||
        (codepoint >= 0x2E80 && codepoint <= 0xA4CF && codepoint != 0x303F) ||
        (codepoint >= 0xAC00 && codepoint <= 0xD7A3) ||
        (codepoint >= 0xF900 && codepoint <= 0xFAFF) ||
        (codepoint >= 0xFE10 && codepoint <= 0xFE19) ||
        (codepoint >= 0xFE30 && codepoint <= 0xFE6F) ||
        (codepoint >= 0xFF01 && codepoint <= 0xFF60) ||
        (codepoint >= 0xFFE0 && codepoint <= 0xFFE6)) return -1;
    return 0;
}

static uint8 qb_unicode_to_screen_byte(uint32 character){
    if (character <= 0x7F) return static_cast<uint8>(character);
    const uint32 mapped=unicode_to_cp437(character);
    if (mapped <= 0xFF) return static_cast<uint8>(mapped);
    return '?';
}

static void qb_normalize_text_cell_state(int32 image_index,int32 cell_index){
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cells.size())) return;

    if (cell_state[cell_index] == 3 && cell_index > 0 && cell_state[cell_index - 1] == 2){
        cells[cell_index - 1].clear();
        cell_state[cell_index - 1]=0;
    }
    if (cell_state[cell_index] == 2 && cell_index + 1 < static_cast<int32>(cells.size()) && cell_state[cell_index + 1] == 3){
        cells[cell_index + 1].clear();
        cell_state[cell_index + 1]=0;
    }

    cells[cell_index].clear();
    cell_state[cell_index]=0;
}

static int32 qb_text_cluster_start(int32 image_index,int32 cell_index){
    if (image_index < 0 || image_index >= nimg) return cell_index;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return cell_index;
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cell_state.size())) return cell_index;
    if (cell_state[cell_index] == 3 && cell_index > 0) return cell_index - 1;
    return cell_index;
}

static int32 qb_text_cell_span(int32 image_index,int32 cell_index){
    if (image_index < 0 || image_index >= nimg) return 1;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return 1;
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cell_state.size())) return 1;
    if (cell_state[cell_index] == 2) return 2;
    if (cell_state[cell_index] == 3) return 0;
    return 1;
}

static void qb_store_text_cell_utf8(img_struct *im,int32 cell_index,uint32 character,int32 cell_span){
    const int32 image_index=qb_img_index(im);
    if (image_index < 0 || image_index >= nimg) return;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    if (cell_index < 0 || cell_index >= static_cast<int32>(cells.size())) return;

    qb_normalize_text_cell_state(image_index,cell_index);
    if (cell_span > 1 && cell_index + 1 < static_cast<int32>(cells.size())) qb_normalize_text_cell_state(image_index,cell_index + 1);

    if (character <= 0x7F || unicode_to_cp437(character)){
        cell_state[cell_index]=0;
        return;
    }

    qb_append_utf8_codepoint(cells[cell_index],character);
    cell_state[cell_index]=(cell_span > 1) ? 2 : 1;
    if (cell_span > 1 && cell_index + 1 < static_cast<int32>(cells.size())){
        cells[cell_index + 1].clear();
        cell_state[cell_index + 1]=3;
    }
}

static int32 qb_append_text_cell_utf8(img_struct *im,int32 cell_index,uint32 character){
    const int32 image_index=qb_img_index(im);
    if (image_index < 0 || image_index >= nimg) return 0;
    if (static_cast<size_t>(image_index) >= qb_text_utf8_cells.size()) return 0;
    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
    cell_index=qb_text_cluster_start(image_index,cell_index);
    if (cell_index < 0 || cell_index >= static_cast<int32>(cells.size())) return 0;
    if (cells[cell_index].empty()){
        const uint8 raw_character=im->offset[cell_index<<1];
        if (raw_character != 32) qb_append_utf8_codepoint(cells[cell_index],codepage437_to_unicode16[raw_character]);
        cell_state[cell_index]=1;
    }
    qb_append_utf8_codepoint(cells[cell_index],character);
    return 1;
}

