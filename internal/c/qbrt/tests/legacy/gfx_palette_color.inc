// Verbatim copy of legacy internal/c/libqb.cpp (commit 4aa6f1f; this code is unchanged from the oracle 545b859), lines 8015-8270.
// Test-only reference oracle for tests/diff_gfx.cpp; production code is internal/c/qbrt/src/gfx.
void qbg_palette(uint32 attribute,uint32 col,int32 passed){
    static int32 r,g,b;
    if (new_error) return;
    if (!passed){restorepalette(write_page); return;}
    
    //32-bit
    if (write_page->bytes_per_pixel==4) goto error;
    
    attribute&=255;//patch to support QBASIC overflow "bug"
    
    if ((write_page->compatible_mode==13)||(write_page->compatible_mode==256)){
        if (col&0xFFC0C0C0) goto error;//11111111110000001100000011000000b
        r=col&63; g=(col>>8)&63; b=(col>>16)&63;
        r=qbr((double)r*4.063492f-0.4999999f); g=qbr((double)g*4.063492f-0.4999999f); b=qbr((double)b*4.063492f-0.4999999f);
        write_page->pal[attribute]=b+g*256+r*65536;
        //Upgraded from (((col<<2)&0xFF)<<16)+(((col>>6)&0xFF)<<8)+((col>>14)&0xFF)
        return;
    }
    
    if (write_page->compatible_mode==12){
        if (attribute>15) goto error;
        if (col&0xFFC0C0C0) goto error;//11111111110000001100000011000000b
        r=col&63; g=(col>>8)&63; b=(col>>16)&63;
        r=qbr((double)r*4.063492f-0.4999999f); g=qbr((double)g*4.063492f-0.4999999f); b=qbr((double)b*4.063492f-0.4999999f);
        write_page->pal[attribute]=b+g*256+r*65536;
        return;
    }
    
    if (write_page->compatible_mode==11){
        if (attribute>1) goto error;
        if (col&0xFFC0C0C0) goto error;//11111111110000001100000011000000b
        r=col&63; g=(col>>8)&63; b=(col>>16)&63;
        r=qbr((double)r*4.063492f-0.4999999f); g=qbr((double)g*4.063492f-0.4999999f); b=qbr((double)b*4.063492f-0.4999999f);
        write_page->pal[attribute]=b+g*256+r*65536;
        return;
    }
    
    if (write_page->compatible_mode==10){
        if (attribute>3) goto error;
        if ((col<0)||(col>8)) goto error;
        write_page->pal[attribute+4]=col;
        return;
    }
    
    if (write_page->compatible_mode==9){
        if (attribute>15) goto error;
        if ((col<0)||(col>63)) goto error;
        write_page->pal[attribute]=palette_64[col];
        return;
    }
    
    if (write_page->compatible_mode==8){
        if (attribute>15) goto error;
        if ((col<0)||(col>15)) goto error;
        write_page->pal[attribute]=palette_256[col];
        return;
    }
    
    if (write_page->compatible_mode==7){
        if (attribute>15) goto error;
        if ((col<0)||(col>15)) goto error;
        write_page->pal[attribute]=palette_256[col];
        return;
    }
    
    if (write_page->compatible_mode==2){
        if (attribute>1) goto error;
        if ((col<0)||(col>15)) goto error;
        write_page->pal[attribute]=palette_256[col];
        return;
    }
    
    if (write_page->compatible_mode==1){
        if (attribute>15) goto error;
        if ((col<0)||(col>15)) goto error;
        write_page->pal[attribute]=palette_256[col];
        return;
    }
    
    if (write_page->compatible_mode==0){
        if (attribute>15) goto error;
        if ((col<0)||(col>63)) goto error;
        write_page->pal[attribute]=palette_64[col];
        return;
    }
    
    error:
    error(5);
    return;
    
}




void qbg_sub_color(uint32 col1,uint32 col2,uint32 bordercolor,int32 passed){
    if (new_error) return;
    if (!passed){
        //performs no action if nothing passed (as in QBASIC for some modes)
        return;
    }

	if (write_page->console){
		#ifdef QBNex_WINDOWS
			HANDLE output = GetStdHandle(STD_OUTPUT_HANDLE);
			int color = col2 * 16 + col1;
			SetConsoleTextAttribute(output, color);
		#else
            // Exactly how the colour is rendered depends on your terminal emulator and
            // colour palette. Themes and user-customisation aside, the first 16 colours
            // line up with the old VGA colour scheme.
            // Most terminal emulators can handle 8 bit colour, see
            // https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit for the 8 bit colour palette.
            if ((passed & 1 && (col1 > 255 || col1 < 0))
                    || (passed & 2 && (col2 > 255 || col2 < 0)))
                goto error;

            if (passed & 1)
                printf("\033[38;5;%dm", col1);

            if (passed & 2)
                printf("\033[48;5;%dm", col2);
		#endif
        return;
	}
    
    if (write_page->compatible_mode==32){
        if (passed&4) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->background_color=col2;
        return;
    }
    if (write_page->compatible_mode==256){
        if (passed&4) goto error;
        if (passed&1) if (col1>255) goto error;
        if (passed&2) if (col2>255) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->background_color=col2;
        return;
    }
    if (write_page->compatible_mode==13){
        //if (passed&6) goto error;
        //if (col1>255) goto error;
        //write_page->color=col1;
        if (passed&4) goto error;
        if (passed&1) if (col1>255) goto error;
        if (passed&2) if (col2>255) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->background_color=col2;
        return;
    }
    if (write_page->compatible_mode==12){
        //if (passed&6) goto error;
        //if (col1>15) goto error;
        //write_page->color=col1;
        if (passed&4) goto error;
        if (passed&1) if (col1>15) goto error;
        if (passed&2) if (col2>15) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->background_color=col2;
        return;
    }
    if (write_page->compatible_mode==11){
        //if (passed&6) goto error;
        //if (col1>1) goto error;
        //write_page->color=col1;
        if (passed&4) goto error;
        if (passed&1) if (col1>1) goto error;
        if (passed&2) if (col2>1) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->background_color=col2;
        return;
    }
    if (write_page->compatible_mode==10){
        if (passed&4) goto error;
        if (passed&1) if (col1>3) goto error;
        if (passed&2) if (col2>8) goto error;
        if (passed&1) write_page->color=col1;
        //if (passed&2) ..._color_assign[0]=col2;
        if (passed&2) write_page->pal[4]=col2;
        return;
    }
    if (write_page->compatible_mode==9){
        if (passed&4) goto error;
        if (passed&1) if (col1>15) goto error;
        if (passed&2) if (col2>63) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->pal[0]=palette_64[col2];
        return;
    }
    if (write_page->compatible_mode==8){
        if (passed&4) goto error;
        if (passed&1) if (col1>15) goto error;
        if (passed&2) if (col2>15) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->pal[0]=palette_256[col2];
        return;
    }
    if (write_page->compatible_mode==7){
        if (passed&4) goto error;
        if (passed&1) if (col1>15) goto error;
        if (passed&2) if (col2>15) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->pal[0]=palette_256[col2];
        return;
    }
    if (write_page->compatible_mode==2){
        if (passed&4) goto error;
        if (passed&1) if (col1>1) goto error;
        if (passed&2) if (col2>15) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->pal[0]=palette_256[col2];
        return;
    }
    if (write_page->compatible_mode==1){
        if (passed&4) goto error;
        if (passed&1){
            if (col1>15) goto error;
            write_page->pal[0]=palette_256[col1];
        }
        if (passed&2){
            if (col2&1){
                write_page->pal[1]=palette_256[3];
                write_page->pal[2]=palette_256[5];
                write_page->pal[3]=palette_256[7];
                }else{
                write_page->pal[1]=palette_256[2];
                write_page->pal[2]=palette_256[4];
                write_page->pal[3]=palette_256[6];
            }
        }
        return;
    }
    if (write_page->compatible_mode==0){
        if (passed&1) if (col1>31) goto error;
        if (passed&2) if (col2>15) goto error;
        if (passed&1) write_page->color=col1;
        if (passed&2) write_page->background_color=col2&7;
        return;
    }
    error:
    error(5);
    return;
}

void defaultcolors(){
    write_page->color=15; write_page->background_color=0;
    if (write_page->compatible_mode==0){write_page->color=7; write_page->background_color=0;}
    if (write_page->compatible_mode==1){write_page->color=3; write_page->background_color=0;}
    if (write_page->compatible_mode==2){write_page->color=1; write_page->background_color=0;}
    if (write_page->compatible_mode==10){write_page->color=3; write_page->background_color=0;}
    if (write_page->compatible_mode==11){write_page->color=1; write_page->background_color=0;}
    if (write_page->compatible_mode==32){write_page->color=0xFFFFFFFF; write_page->background_color=0xFF000000;}
    write_page->draw_color=write_page->color;
    return;
}
