// Verbatim copy of legacy internal/c/libqb.cpp (commit 4aa6f1f; this code is unchanged from the oracle 545b859), lines 3694-3782 3787-3829.
// Test-only reference oracle for tests/diff_gfx.cpp; production code is internal/c/qbrt/src/gfx.
void restorepalette(img_struct* im){
    static uint32 *pal;
    if (im->bytes_per_pixel==4) return;
    pal=im->pal;
    
    switch(im->compatible_mode){
        
        case 1:
        /*
            SCREEN Mode 1 Syntax:  COLOR [background][,palette]
            \A6 background is the screen color (range = 0-15)
            \A6 palette is a three-color palette (range = 0-1)
            0 = green, red, and brown         1 = cyan, magenta, and bright white
            Note: option 1 is the default, palette can override these though
            OPTION 1:*DEFAULT*
            0=black(color 0)
            1=cyan(color 3)
            2=purple(color 5)
            3=light grey(color 7)
            OPTION 0:
            0=black(color 0)
            1=green(color 2)
            2=red(color 4)
            3=brown(color 6)
        */
        pal[0]=palette_256[0];
        pal[1]=palette_256[3];
        pal[2]=palette_256[5];
        pal[3]=palette_256[7];
        return;
        break;
        
        case 2://black/white 2 color palette
        pal[0]=0;
        pal[1]=0xFFFFFF;
        return;
        break;
        
        case 9://16 colors selected from 64 possibilities
        pal[0]=palette_64[0];
        pal[1]=palette_64[1];
        pal[2]=palette_64[2];
        pal[3]=palette_64[3];
        pal[4]=palette_64[4];
        pal[5]=palette_64[5];
        pal[6]=palette_64[20];
        pal[7]=palette_64[7];
        pal[8]=palette_64[56];
        pal[9]=palette_64[57];
        pal[10]=palette_64[58];
        pal[11]=palette_64[59];
        pal[12]=palette_64[60];
        pal[13]=palette_64[61];
        pal[14]=palette_64[62];
        pal[15]=palette_64[63];
        return;
        break;
        
        case 10://4 colors selected from 9 possibilities (indexes held in array pal[4-7])
        pal[4]=0;
        pal[5]=4;
        pal[6]=6;
        pal[7]=8;
        return;
        break;
        
        case 11://black/white 2 color palette
        pal[0]=0;
        pal[1]=0xFFFFFF;
        return;
        break;
        
        case 13:
        memcpy(pal,palette_256,1024);
        return;
        break;
        
        case 256:
        memcpy(pal,palette_256,1024);
        return;
        break;
        
        default:
        //default 16 color palette
        memcpy(pal,palette_256,64);
        
    };//switch
    
}//restorepalette
void pset(int32 x,int32 y,uint32 col){
    static uint8 *cp;
    static uint32 *o32;
    static uint32 destcol;
    if (write_page->bytes_per_pixel==1){
        write_page->offset[y*write_page->width+x]=col&write_page->mask;
        return;
        }else{
        if (write_page->alpha_disabled){
            write_page->offset32[y*write_page->width+x]=col;
            return;
        }
        switch(col&0xFF000000){
            case 0xFF000000://100% alpha, so regular pset (fast)
            write_page->offset32[y*write_page->width+x]=col;
            return;
            break;
            case 0x0://0%(0) alpha, so no pset (very fast)
            return;
            break;
            case 0x80000000://~50% alpha (optomized)
            
            o32=write_page->offset32+(y*write_page->width+x);
            *o32=(((*o32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend128[*o32>>24]<<24);
            return;
            break; 
            case 0x7F000000://~50% alpha (optomized)
            o32=write_page->offset32+(y*write_page->width+x);
            *o32=(((*o32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend127[*o32>>24]<<24);
            return;
            break;
            default://other alpha values (uses a lookup table)
            o32=write_page->offset32+(y*write_page->width+x);
            destcol=*o32;
            cp=cblend+(col>>24<<16);
            *o32=
            cp[(col<<8&0xFF00)+(destcol&255)    ]
            +(cp[(col&0xFF00)   +(destcol>>8&255) ]<<8)
            +(cp[(col>>8&0xFF00)+(destcol>>16&255)]<<16)
            +(ablend[(col>>24)+(destcol>>16&0xFF00)]<<24);
        };
    }
}
