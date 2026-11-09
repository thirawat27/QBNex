// Verbatim from internal/c/libqb.cpp (R12a): printchr, chrwidth, newline, makefit, tab, qbs_print, VIEW PRINT.
int32 img_printchr=0;
int32 img_printchr_i;
int32 img_printchr_x;
int32 img_printchr_y;
char *img_printchr_offset;

void printchr(int32 character,int32 cell_span){
    static uint32 x,x2,y,y2,w,h,z,z2,z3,a,a2,a3,color,background_color,f;
    static uint32 *lp;
    static uint8 *cp;
    static img_struct *im;
    
    im=write_page;
    color=im->color;
    background_color=im->background_color;
    
    
    if (im->text){
        const int32 cell_index=(im->cursor_y-1)*im->width+im->cursor_x-1;
        im->offset[cell_index<<1]=qb_unicode_to_screen_byte(character);
        im->offset[(cell_index<<1)+1]=(color&0xF)+background_color*16+(color&16)*8;
        if (cell_span > 1 && cell_index + 1 < im->width * im->height){
            im->offset[(cell_index+1)<<1]=32;
            im->offset[((cell_index+1)<<1)+1]=(color&0xF)+background_color*16+(color&16)*8;
        }
        qb_store_text_cell_utf8(im,cell_index,character,cell_span);
        return;
    }
    
    //precalculations
    
    f=im->font;
    x=fontwidth[f]; if (x) x*=(im->cursor_x-1); else x=im->cursor_x-1;
    y=(im->cursor_y-1)*fontheight[f];
    h=fontheight[f];
    if ((fontflags[f]&32)==0) character&=255;//unicodefontsupport
    
    //if (mode==1) img[i].print_mode=3;//fill
    //if (mode==2) img[i].print_mode=1;//keep
    //if (mode==3) img[i].print_mode=2;//only
    
    if (f>=32){//custom font
        
        //8-bit / alpha-disabled 32-bit / dont-blend(alpha may still be applied)
        if ((im->bytes_per_pixel==1)||((im->bytes_per_pixel==4)&&(im->alpha_disabled))||(fontflags[f]&8)){
            
            //render character
            static int32 ok;
            static uint8 *rt_data;
            static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
            //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
            //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
            ok=FontRenderTextASCII(font[f],(uint8*)&character,1,1,
            &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
            if (!ok) return;
            
            w=rt_w;
            
            switch(im->print_mode){
                case 3:
                for (y2=0;y2<h;y2++){
                    cp=rt_data+y2*w;
                    for (x2=0;x2<w;x2++){
                        if (*cp++) pset(x+x2,y+y2,color); else pset(x+x2,y+y2,background_color);
                    }}
                    break;
                    case 1:
                    for (y2=0;y2<h;y2++){
                        cp=rt_data+y2*w;
                        for (x2=0;x2<w;x2++){
                            if (*cp++) pset(x+x2,y+y2,color);
                        }}
                        break;
                        case 2:
                        for (y2=0;y2<h;y2++){
                            cp=rt_data+y2*w;
                            for (x2=0;x2<w;x2++){
                                if (!(*cp++)) pset(x+x2,y+y2,background_color);
                            }}
                            break;
                            default:
                            break;
            }
            
            free(rt_data);
            return;
        }//1-8 bit
        //assume 32-bit blended
        
        a=(color>>24)+1; a2=(background_color>>24)+1;
        z=color&0xFFFFFF; z2=background_color&0xFFFFFF;
        
        //render character
        static int32 ok;
        static uint8 *rt_data;
        static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
        //int32 FontRenderTextASCII(int32 i,uint8*codepoint,int32 codepoints,int32 options,
        //                          uint8**out_data,int32*out_x,int32 *out_y,int32*out_x_pre_increment,int32*out_x_post_increment){
        ok=FontRenderTextASCII(font[f],(uint8*)&character,1,0,
        &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
        if (!ok) return;
        
        w=rt_w;
        
        switch(im->print_mode){
            case 3:
            
            static float r1,g1,b1,alpha1,r2,g2,b2,alpha2;
            alpha1=(color>>24)&255; r1=(color>>16)&255; g1=(color>>8)&255; b1=color&255;
            alpha2=(background_color>>24)&255; r2=(background_color>>16)&255; g2=(background_color>>8)&255; b2=background_color&255;
            static float dr,dg,db,da;
            dr=r2-r1;
            dg=g2-g1;
            db=b2-b1;
            da=alpha2-alpha1;
            static float cw;//color weight multiplier, avoids seeing black when transitioning from RGBA(?,?,?,255) to RGBA(0,0,0,0)
            if (alpha1) cw=alpha2/alpha1; else cw=100000;
            static float d;
            
            for (y2=0;y2<h;y2++){
                cp=rt_data+y2*w;
                for (x2=0;x2<w;x2++){
                    
                    d=*cp++;
                    d=255-d;
                    d/=255.0;
                    static float r3,g3,b3,alpha3;
                    alpha3=alpha1+da*d;
                    d*=cw; if (d>1.0) d=1.0;
                    r3=r1+dr*d;
                    g3=g1+dg*d;
                    b3=b1+db*d;
                    static int32 r4,g4,b4,alpha4;
                    r4=qbr_float_to_long(r3);
                    g4=qbr_float_to_long(g3);
                    b4=qbr_float_to_long(b3);
                    alpha4=qbr_float_to_long(alpha3);
                    pset(x+x2,y+y2,b4+(g4<<8)+(r4<<16)+(alpha4<<24));
                    
                }}
                break;
                case 1:
                for (y2=0;y2<h;y2++){
                    cp=rt_data+y2*w;
                    for (x2=0;x2<w;x2++){
                        z3=*cp++;
                        if (z3) pset(x+x2,y+y2,((z3*a)>>8<<24)+z);
                    }}
                    break;
                    case 2:
                    for (y2=0;y2<h;y2++){
                        cp=rt_data+y2*w;
                        for (x2=0;x2<w;x2++){
                            z3=*cp++;
                            if (z3!=255) pset(x+x2,y+y2,(((255-z3)*a2)>>8<<24)+z2);
                        }}
                        break;
                        default:
                        break;
        }
        free(rt_data);
        return;
    }//custom font
    
    
    //default fonts (8x8, 8x14, 8x16): Rust (qb-runtime gfx/text.rs)
    qb_gfx_printchr(im,character);
    return;
}//printchr





int32 chrwidth(uint32 character){
    //Note: Only called by qbs_print()
    //      Supports "UNICODE" _LOADFONT option
    static int32 w;
    static img_struct *im;
    im=write_page;
    if (w=fontwidth[im->font]) return w;
    
    //Custom font
    static int32 render_option,f;
    static int32 ok;
    static uint8 *rt_data;
    static int32 rt_w,rt_h,rt_pre_x,rt_post_x;
    
    f=im->font;
    
    render_option=0;
    //8-bit / alpha-disabled 32-bit / dont-blend(alpha may still be applied)
    if ((im->bytes_per_pixel==1)||((im->bytes_per_pixel==4)&&(im->alpha_disabled))||(fontflags[f]&8)){
        render_option=1;
    }
    
    if ((fontflags[f]&32)){//UNICODE character
        ok=FontRenderTextUTF32(font[f],(uint32*)&character,1,render_option,
        &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
        }else{//ASCII character
        character&=255;
        ok=FontRenderTextASCII(font[f],(uint8*)&character,1,render_option,
        &rt_data,&rt_w,&rt_h,&rt_pre_x,&rt_post_x);
    }
    if (!ok) return 0;
    free(rt_data);
    return rt_w;
    
}//chrwidth


void newline(){
    static uint32 *lp;
    static uint16 *sp;
    static int32 z,z2;
    
    //move cursor to new line
    write_page->cursor_y++; write_page->cursor_x=1;
    
    //scroll up screen if necessary
    if (write_page->cursor_y>write_page->bottom_row){
        
        if (lprint){
            sub__printimage(lprint_image);
            sub_cls(NULL,15,2);
            lprint_buffered=0;
            return;
        }
        
        if (write_page->text){
            //text
            //move lines up
            memmove(
            write_page->offset+(write_page->top_row-1)*2*write_page->width,
            write_page->offset+ write_page->top_row   *2*write_page->width,
            (write_page->bottom_row-write_page->top_row)*2*write_page->width
            );
            {
                const int32 image_index=qb_img_index(write_page);
                if (image_index >= 0 && image_index < nimg && image_index < qb_text_utf8_cells.size()){
                    std::vector<std::string> &cells=qb_text_utf8_cells[image_index];
                    std::vector<uint8> &cell_state=qb_text_utf8_cell_state[image_index];
                    if (!cells.empty()){
                        const int32 row_width=write_page->width;
                        const int32 dst=(write_page->top_row-1)*row_width;
                        const int32 src=write_page->top_row*row_width;
                        const int32 moved=(write_page->bottom_row-write_page->top_row)*row_width;
                        for (int32 cell=0;cell<moved;cell++){
                            cells[dst+cell]=cells[src+cell];
                            cell_state[dst+cell]=cell_state[src+cell];
                        }
                        qb_clear_text_utf8_cells_range(image_index,(write_page->bottom_row-1)*row_width,row_width);
                    }
                }
            }
            //erase bottom line
            z2=(write_page->color&0xF)+(write_page->background_color&7)*16+(write_page->color&16)*8;
            z2<<=8;
            z2+=32;
            sp=((uint16*)(write_page->offset+(write_page->bottom_row-1)*2*write_page->width));
            z=write_page->width;
            while(z--) *sp++=z2;
            }else{
            //graphics
            //move lines up
            memmove(
            write_page->offset+(write_page->top_row-1)*write_page->bytes_per_pixel*write_page->width*fontheight[write_page->font],
            write_page->offset+ write_page->top_row   *write_page->bytes_per_pixel*write_page->width*fontheight[write_page->font],
            (write_page->bottom_row-write_page->top_row)*write_page->bytes_per_pixel*write_page->width*fontheight[write_page->font]
            );
            //erase bottom line
            if (write_page->bytes_per_pixel==1){
                memset(write_page->offset+(write_page->bottom_row-1)*write_page->width*fontheight[write_page->font],write_page->background_color,write_page->width*fontheight[write_page->font]);
                }else{
                //assume 32-bit
                z2=write_page->background_color;
                lp=write_page->offset32+(write_page->bottom_row-1)*write_page->width*fontheight[write_page->font];
                z=write_page->width*fontheight[write_page->font];
                while(z--) *lp++=z2;
            }
        }//graphics
        write_page->cursor_y=write_page->bottom_row;
    }//scroll up
    
}//newline

void makefit(qbs *text){
    static int32 w,x,x2,x3;
    if (write_page->holding_cursor) return;
    if (write_page->cursor_x!=1){//if already at left-most, nothing more can be done
        if (write_page->text){
            if ((write_page->cursor_x+text->len-1)>write_page->width) newline();
            }else{
            w=func__printwidth(text,NULL,NULL);
            x=fontwidth[write_page->font]; if (!x) x=1; x=x*(write_page->cursor_x-1);
            if ((x+w)>write_page->width) newline();
        }
    }
}

void lprint_makefit(qbs *text){
    //stub
    //makefit(text);
}

void tab(){
    static int32 x,x2,w;
    
    //tab() on a held-cursor only sets the cursor to the left hand position of the next line
    if (write_page->holding_cursor){
        newline(); write_page->holding_cursor=0;
        return;
    }

    #ifdef QBNex_WINDOWS //if Windows console
        if (write_page->console){
            if (func_pos(0)>write_page->width-10){
                printf("\n");
                return;
            }else{
                do {
                    printf(" ");
                }while(func_pos(0) % 10!=0);
            }
            return;
        }
    #endif
    
    //text
    if (write_page->text){
        qbs_print(singlespace,0);
        text:
        if (write_page->cursor_x!=1){
            if (((write_page->cursor_x-1)%14)||(write_page->cursor_x>(write_page->width-13))){
                if (write_page->cursor_x<write_page->width){qbs_print(singlespace,0); goto text;}
            }
        }//!=1
        return;
    }
    
    x=fontwidth[write_page->font]; 
    if (!x){
        
        //variable width
        x=write_page->cursor_x-1;
        x2=(x/112+1)*112;//next position
        if (x2>=write_page->width){//it doesn't fit on line
            //box fill x to end of line with background color
            fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],write_page->width-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
            newline();
            }else{//fits on line
            //box fill x to x2-1 with background color
            fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],x2-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
            write_page->cursor_x=x2;
        }
        
        
        }else{
        
        //fixed width
        w=write_page->width/x;
        
        qbs_print(singlespace,0);
        fixwid:
        if (write_page->cursor_x!=1){
            if (((write_page->cursor_x-1)%14)||(write_page->cursor_x>(w-13))){
                if (write_page->cursor_x<w){qbs_print(singlespace,0); goto fixwid;}
            }
        }//!=1
        
    }
    return;
}

int32 func_lpos(int32 lpt){
    //lpt values: 0 = LPT1, 1 = LPT1, 2 = LPT2, 3 = LPT3
    if ((lpt<0)||(lpt>3)){error(5); return 0;}
    return lpos;
}

void qbs_lprint(qbs* str,int32 finish_on_new_line){
    static int32 old_dest;
    while (lprint_locked) Sleep(64);
    lprint=1;
    old_dest=func__dest();
    if (!lprint_image){
        lprint_image=func__newimage(640,960,13,1);
        sub__dest(lprint_image);
        sub_cls(NULL,15,2);
        sub__font(16,NULL,0);
        qbg_sub_color(0,15,NULL,3);
        qbg_sub_view_print(1,60,1);
        }else{
        sub__dest(lprint_image);
    }
    lprint_buffered=1;
    lprint_last=func_timer(0.001,1);
    qbs_print(str,finish_on_new_line);
    sub__dest(old_dest);
    lprint=0;
}

int32 no_control_characters=0;
int32 no_control_characters2=0;

void sub__controlchr(int32 onoff){
    if (onoff==2) no_control_characters2=1; else no_control_characters2=0;
}

int32 func__controlchr () {
    return -no_control_characters2;
}

void qbs_print(qbs* str,int32 finish_on_new_line){
    if (new_error) return;
    int32 i,i2,entered_new_line,x,x2,y,y2,z,z2,w,cell_span;
    entered_new_line=0;
    static uint32 character;
    
    if (write_page->console){
        qb_write_utf8(reinterpret_cast<const char*>(str->chr), str->len, finish_on_new_line);
        return;
    }
    
    /*
        if (!str->len){
        if (!newline) return;//no action required
        if (write_page->holding_cursor){//extra CR required before return
        write_page->holding_cursor=0;
        i=-1;
        write_page->cursor_x++;
        goto print_unhold_cursor;
        }
        }
        
        if (!str->len) goto null_length;
        
        if (write_page->holding_cursor){
        write_page->holding_cursor=0;
        i=-1;
        write_page->cursor_x++;
        goto print_unhold_cursor;
        }
    */
    
    //holding cursor?
    if (write_page->holding_cursor){
        if (str->len){
            write_page->holding_cursor=0;
            newline();
            }else{
            //null length print string
            if (finish_on_new_line) write_page->holding_cursor=0;//new line will be entered automatically
        }
    }
    
    
    for (i=0;i<str->len;i++){
        
        character=(uint8)str->chr[i];
        
        if (write_page->text){
            const int32 bytes_used=qb_decode_utf8_codepoint((const uint8*)&str->chr[i],str->len-i,&character);
            if (!bytes_used) break;
            i+=bytes_used-1;
        }else if (fontflags[write_page->font]&32){//unicode font
            if (i>(str->len-4)) break;//not enough data for a utf32 encoding
            character=*((int32*)(&str->chr[i]));
            i+=3;
        }
        
        if (lprint) lprint_buffered=1;
        entered_new_line=0;//beginning a new line was the last action (so don't add a new one)
        
        
        
        //###special characters
        
        if (no_control_characters||no_control_characters2) goto skip_control_characters;
        
        if (character==28){
            //advance one cursor position
            if (lprint){
                if (lpos<width_lprint) lpos++;
            }
            //can cursor advance?
            if (write_page->cursor_y>=write_page->bottom_row){
                if (write_page->text){
                    if (write_page->cursor_x>=write_page->width) goto skip;
                    }else{
                    if (fontwidth[write_page->font]){
                        if (write_page->cursor_x>=(write_page->width/fontwidth[write_page->font])) goto skip;
                        }else{
                        if (write_page->cursor_x>=write_page->width) goto skip;
                    }
                } 
            }
            write_page->cursor_x++;
            if (write_page->text){
                if (write_page->cursor_x>write_page->width){write_page->cursor_y++; write_page->cursor_x=1;}
                }else{
                if (fontwidth[write_page->font]){
                    if (write_page->cursor_x>(write_page->width/fontwidth[write_page->font])){write_page->cursor_y++; write_page->cursor_x=1;}
                    }else{
                    if (write_page->cursor_x>write_page->width){write_page->cursor_y++; write_page->cursor_x=1;}
                }
            } 
            goto skip;
        }
        
        if (character==29){
            //go back one cursor position
            if (lprint){
                if (lpos>1) lpos--;
            }
            //can cursor go back?
            if ((write_page->cursor_y==write_page->top_row)||(write_page->cursor_y>write_page->bottom_row)){
                if (write_page->cursor_x==1) goto skip;
            }
            write_page->cursor_x--;
            if (write_page->cursor_x<1){
                write_page->cursor_y--;
                if (write_page->text){
                    write_page->cursor_x=write_page->width;
                    }else{
                    if (fontwidth[write_page->font]){
                        write_page->cursor_x=write_page->width/fontwidth[write_page->font];
                        }else{
                        write_page->cursor_x=write_page->width;
                    }
                } 
            }
            goto skip;
        }
        
        if (character==30){
            //previous row, same column
            //no change if cursor not within view print boundries
            if ((write_page->cursor_y>write_page->top_row)&&(write_page->cursor_y<=write_page->bottom_row)){
                write_page->cursor_y--;
            }
            goto skip;
        }
        
        if (character==31){
            //next row, same column
            //no change if cursor not within view print boundries
            if ((write_page->cursor_y>=write_page->top_row)&&(write_page->cursor_y<write_page->bottom_row)){
                write_page->cursor_y++;
            }
            goto skip;
        }
        
        if (character==12){//aka form feed
            if (lprint){sub__printimage(lprint_image); lprint_buffered=0;}
            //clears text viewport
            //clears bottom row
            //moves cursor to top-left of text viewport
            sub_cls(NULL,NULL,0);
            if (lprint) lpos=1;
            goto skip;
        }
        
        if (character==11){
            write_page->cursor_x=1; write_page->cursor_y=write_page->top_row;
            if (lprint) lpos=1;
            goto skip;
        }
        
        if (character==9){
            //moves to next multiple of 8 (always advances at least one space)
            if (!fontwidth[write_page->font]){
                //variable width!
                x=write_page->cursor_x-1;
                x2=(x/64+1)*64;//next position
                if (x2>=write_page->width){//it doesn't fit on line
                    //box fill x to end of line with background color
                    fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],write_page->width-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
                    newline();
                    entered_new_line=1;
                    }else{//fits on line
                    //box fill x to x2-1 with background color
                    fast_boxfill(x,(write_page->cursor_y-1)*fontheight[write_page->font],x2-1,write_page->cursor_y*fontheight[write_page->font]-1,write_page->background_color);
                    write_page->cursor_x=x2;
                }
                goto skip;
                }else{
                if (write_page->cursor_x%8){//next cursor position not a multiple of 8
                    i--;//more spaces will be required
                }
                character=32;//override character 9
            }
        }//9
        
        if (character==7){
            //qbnex_generatesound(783.99,0.2,0);
            Sleep(250);
            goto skip;
        }
        
        if ((character==10)||(character==13)){
            newline();
            if (lprint) lpos=1;
            //note: entered_new_line not set because these carriage returns compound on each other
            goto skip;
        }
        
        skip_control_characters:
        
        if (write_page->text && qb_is_zero_width_codepoint(character)){
            int32 previous_cell=-1;
            if (write_page->cursor_x > 1){
                previous_cell=(write_page->cursor_y-1)*write_page->width+write_page->cursor_x-2;
            }else if (write_page->cursor_y > write_page->top_row){
                previous_cell=(write_page->cursor_y-2)*write_page->width+write_page->width-1;
            }
            if (previous_cell >= 0 && qb_append_text_cell_utf8(write_page,previous_cell,character)){
                goto skip;
            }
        }

        cell_span=1;
        if (write_page->text && qb_is_wide_codepoint(character)) cell_span=2;
        if (write_page->text && cell_span > 1){
            if ((write_page->cursor_x + cell_span - 1) > write_page->width){
                newline();
            }
        }
        
        //###check if character fits on line, if not move to next line
        //(only applies to non-fixed width fonts)
        if (!fontwidth[write_page->font]){//unpredictable width
            w=chrwidth(character);
            if ((write_page->cursor_x+w)>write_page->width){
                newline();
                //entered_new_line not set, a character will follow
            }
        }
        
        //###print the character
        printchr(character,cell_span);
        
        //###advance lpos, begin new line if necessary
        if (lprint){
            lpos++;
            if (lpos>width_lprint){
                newline();
                entered_new_line=1;
                lpos=1;
                goto skip;//skip cursor advancement and checking because new line entered
            }
        }
        
        //###advance cursor
        if (fontwidth[write_page->font]){
            write_page->cursor_x+=cell_span;
            }else{
            write_page->cursor_x+=w;
        }
        
        //###check if another character could fit at cursor_x's location
        if (write_page->compatible_mode){//graphics
            x=fontwidth[write_page->font]; if (!x) x=1;
            x2=x*(write_page->cursor_x-1);
            if (x2>(write_page->width-x)){
                if (!finish_on_new_line){
                    if (i==(str->len-1)){//last character
                        //move horizontal cursor back to right-most valid position
                        write_page->cursor_x=write_page->width/x;
                        write_page->holding_cursor=1;
                        goto held_cursor;
                    }
                }
                newline();
                entered_new_line=1;
            }
            }else{//text
            if (write_page->cursor_x>write_page->width){
                if (!finish_on_new_line){
                    if (i==(str->len-1)){//last character
                        write_page->cursor_x--;//move horizontal cursor back to right-most valid position
                        write_page->holding_cursor=1;
                        goto held_cursor;
                    }
                }
                newline();
                entered_new_line=1;
            }
        }
        held_cursor:
        
        skip:;
        
        /*
            tabbing1:
            
            
            write_page->cursor_x++;
            
            
            
            //hold cursor?
            if (write_page->cursor_x>qbg_width_in_characters){//past last x position
            if (!newline){//don't need a new line
            if (i==(str->len-1)){//last character
            write_page->cursor_x--;
            write_page->holding_cursor=1;
            goto hold_cursor;
            }
            }
            }
            
            
            
            
            qbs_print_skipchar:;
            
            print_unhold_cursor:
            
            if (write_page->cursor_x>qbg_width_in_characters){
            qbs_print_newline:
            newlineadded=1;
            
            if (write_page->cursor_y==qbg_height_in_characters) write_page->cursor_y=qbg_bottom_row;
            
            write_page->cursor_y++;
            write_page->cursor_x=1;
            
            
            
            if (write_page->cursor_y>qbg_bottom_row){
            //move screen space within view print up 1 row
            //if (qbg_mode==13){
            
            ///memmove(&cmem[655360+(qbg_top_row-1)*2560],&cmem[655360+qbg_top_row*2560],(qbg_bottom_row-qbg_top_row)*2560);
            ///memset(&cmem[655360+(qbg_bottom_row-1)*2560],0,2560);
            if (qbg_text_only){
            
            memmove(qbg_active_page_offset+(qbg_top_row-1)*qbg_width_in_characters*2,
            qbg_active_page_offset+(qbg_top_row)*qbg_width_in_characters*2,
            (qbg_bottom_row-qbg_top_row)*qbg_width_in_characters*2);
            for (i2=0;i2<qbg_width_in_characters;i2++){
            qbg_active_page_offset[(qbg_bottom_row-1)*qbg_width_in_characters*2+i2*2]=32;
            qbg_active_page_offset[(qbg_bottom_row-1)*qbg_width_in_characters*2+i2*2+1]=7;
            
            }
            
            }else{
            memmove(qbg_active_page_offset+(qbg_top_row-1)*qbg_bytes_per_pixel*qbg_width*qbg_character_height,
            qbg_active_page_offset+qbg_top_row*qbg_bytes_per_pixel*qbg_width*qbg_character_height,
            (qbg_bottom_row-qbg_top_row)*qbg_bytes_per_pixel*qbg_width*qbg_character_height);
            memset(qbg_active_page_offset+(qbg_bottom_row-1)*qbg_bytes_per_pixel*qbg_width*qbg_character_height,0,qbg_bytes_per_pixel*qbg_width*qbg_character_height);
            }
            
            
            
            
            write_page->cursor_y=qbg_bottom_row;
            }
            
            
            
            }
        */
        
    }//i
    
    null_length:
    if (finish_on_new_line&&(!entered_new_line)) newline();
    if (lprint){
        if (finish_on_new_line) lpos=1;
    }
    
    
    /*
        null_length:
        
        //begin new line?
        if (newline&&(!newlineadded)) {newline=0; goto qbs_print_newline;}
        
        //hold cursor
        hold_cursor:
    */
    
    return;
    
}
void qbg_sub_view_print(int32 topline,int32 bottomline,int32 passed){
    if (new_error) return;
    
    static int32 maxrows;
    maxrows=write_page->height; if (!write_page->text) maxrows/=fontheight[write_page->font];
    
    if (!passed){//topline and bottomline not passed
        write_page->top_row=1; write_page->bottom_row=maxrows;
        write_page->cursor_y=1; write_page->cursor_x=1;
        write_page->holding_cursor=0;
        return;
    }
    
    if (topline<=0) goto error;
    if (topline>maxrows) goto error;
    if (bottomline<topline) goto error;
    if (bottomline>maxrows) goto error;
    
    write_page->top_row=topline;
    write_page->bottom_row=bottomline;
    write_page->cursor_y=write_page->top_row;
    write_page->cursor_x=1;
    write_page->holding_cursor=0;
    return;
    
    error:
    error(5);
    return;
}
