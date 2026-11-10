// Verbatim from internal/c/libqb.cpp (R12a): CLS, LOCATE.
void sub_cls(int32 method,uint32 use_color,int32 passed){
    if (new_error) return;
    static int32 characters,i;
    static uint16 *sp;
    static uint16 clearvalue;
    
	if (write_page->console){ 
		#ifdef QBNex_WINDOWS
			system("cls"); //lazy but works
			qbg_sub_locate(1, 1, 0, 0, 0, 3); //is this really necessary?
		#else
			if (passed&2) qbg_sub_color(0, use_color, 0, 2);
			cout<<"\033[2J";
			qbg_sub_locate(1, 1, 0, 0, 0, 3);
		#endif
		return;
	}

    //validate
    if (passed&2){
        if (write_page->bytes_per_pixel!=4){
            if (use_color>write_page->mask) goto error;
        }
        }else{
        use_color=write_page->background_color;
    }
    
    if (passed&1){
        if ((method>2)||(method<0)) goto error;
    }
    
    
    
    //all CLS methods reset the cursor position
    write_page->cursor_y=write_page->top_row;
    write_page->cursor_x=1;
    
    //all CLS methods reset DRAW attributes
    write_page->draw_ta=0.0; write_page->draw_scale=1.0;
    
    if (write_page->text){
        //precalculate a (int16) value which can be used to clear the screen
        clearvalue=(write_page->color&0xF)+(use_color&7)*16+(write_page->color&16)*8;
        clearvalue<<=8;
        clearvalue+=32;
    }
    
    if ((passed&1)==0){//no method specified
        //video mode: clear only graphics viewport
        //text mode: clear text view port AND the bottom line
        if (write_page->text){
            //text view port
            characters=write_page->width*(write_page->bottom_row-write_page->top_row+1);
            sp=(uint16*)&write_page->offset[(write_page->top_row-1)*write_page->width*2];
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),(write_page->top_row-1)*write_page->width,characters);
            //bottom line
            characters=write_page->width;
            sp=(uint16*)&write_page->offset[(write_page->height-1)*write_page->width*2];
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),(write_page->height-1)*write_page->width,characters);
            key_display_redraw=1; key_update();
            return;
            }else{//graphics
            //graphics view port
            if (write_page->bytes_per_pixel==1){//8-bit
                if (write_page->clipping_or_scaling){
                    qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                    }else{//fast method (no clipping/scaling)
                    memset(write_page->offset,use_color,write_page->width*write_page->height);
                }
                }else{//32-bit
                i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
                if (write_page->clipping_or_scaling){
                    qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                    }else{//fast method (no clipping/scaling)
                    fast_boxfill(0,0,write_page->width-1,write_page->height-1,use_color);
                }
                write_page->alpha_disabled=i;
            }
        }
        
        if (write_page->clipping_or_scaling==2){
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/write_page->scaling_x/2.0f+write_page->scaling_offset_x;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/write_page->scaling_y/2.0f+write_page->scaling_offset_y;
            }else{
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/2.0f;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/2.0f;
        }
        
        key_display_redraw=1; key_update();
        return;
    }
    
    if (method==0){//clear everything
        if (write_page->text){
            characters=write_page->height*write_page->width;
            sp=(uint16*)write_page->offset;
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),0,characters);
            key_display_redraw=1; key_update();
            return;
            }else{
            if (write_page->bytes_per_pixel==1){
                memset(write_page->offset,use_color,write_page->width*write_page->height);
                }else{ 
                i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
                fast_boxfill(0,0,write_page->width-1,write_page->height-1,use_color);
                write_page->alpha_disabled=i;
            }
        }
        
        if (write_page->clipping_or_scaling==2){
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/write_page->scaling_x/2.0f+write_page->scaling_offset_x;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/write_page->scaling_y/2.0f+write_page->scaling_offset_y;
            }else{
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/2.0f;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/2.0f;
        }
        
        key_display_redraw=1; key_update();
        return;
    }
    
    if (method==1){//ONLY clear the graphics viewport
        if (write_page->text) return;
        //graphics view port
        if (write_page->bytes_per_pixel==1){//8-bit
            if (write_page->clipping_or_scaling){
                qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                }else{//fast method (no clipping/scaling)
                memset(write_page->offset,use_color,write_page->width*write_page->height);
            }
            }else{//32-bit
            i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
            if (write_page->clipping_or_scaling){
                qb32_boxfill(write_page->window_x1,write_page->window_y1,write_page->window_x2,write_page->window_y2,use_color);
                }else{//fast method (no clipping/scaling)
                fast_boxfill(0,0,write_page->width-1,write_page->height-1,use_color);
            }
            write_page->alpha_disabled=i;
        }
        
        if (write_page->clipping_or_scaling==2){
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/write_page->scaling_x/2.0f+write_page->scaling_offset_x;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/write_page->scaling_y/2.0f+write_page->scaling_offset_y;
            }else{
            write_page->x=((float)(write_page->view_x2-write_page->view_x1+1))/2.0f;
            write_page->y=((float)(write_page->view_y2-write_page->view_y1+1))/2.0f;
        }
        
        key_display_redraw=1; key_update();
        return;
    }
    
    if (method==2){//ONLY clear the VIEW PRINT range text viewport
        if (write_page->text){
            //text viewport
            characters=write_page->width*(write_page->bottom_row-write_page->top_row+1);
            sp=(uint16*)&write_page->offset[(write_page->top_row-1)*write_page->width*2];
            for (i=0;i<characters;i++){sp[i]=clearvalue;}
            qb_clear_text_utf8_cells_range(qb_img_index(write_page),(write_page->top_row-1)*write_page->width,characters);
            //should not and does not redraw KEY bar
            return;
            }else{
            //text viewport
            if (write_page->bytes_per_pixel==1){//8-bit
                memset(&write_page->offset[write_page->width*fontheight[write_page->font]*(write_page->top_row-1)],use_color,write_page->width*fontheight[write_page->font]*(write_page->bottom_row-write_page->top_row+1));
                }else{//32-bit
                i=write_page->alpha_disabled; write_page->alpha_disabled=1;  
                fast_boxfill(0,fontheight[write_page->font]*(write_page->top_row-1),write_page->width-1,fontheight[write_page->font]*write_page->bottom_row-1,use_color);
                write_page->alpha_disabled=i;
            }
            //should not and does not redraw KEY bar
            return;
        }
    }
    
    return;
    error:
    error(5);
    return;
}


void qbg_sub_locate(int32 row,int32 column,int32 cursor,int32 start,int32 stop,int32 passed){
    static int32 h,w,i;
    if (new_error) return;

	if (write_page->console){
		#ifdef QBNex_WINDOWS //If trying to locate with windows console
			CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
			SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
			HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
			GetConsoleScreenBufferInfo(cl_conout, & cl_bufinfo);
			if (column==0)column=cl_bufinfo.dwCursorPosition.X + 1;
			if (row==0)row=cl_bufinfo.dwCursorPosition.Y + 1;
			COORD pos = {column-1, row-1};
			HANDLE output = GetStdHandle (STD_OUTPUT_HANDLE);
			SetConsoleCursorPosition(output, pos);
		#else
            if (passed & 1 && passed & 2){
                printf("\033[%d;%dH", row, column);
                return;
            }
            if (passed & 1){
                printf("\033[%dd", row);
                return;
            }
            if (passed & 2){
                printf("\033[%d;%dG", column);
                return;
            }
        #endif
        return;
	}

    //calculate height & width in characters
    if (write_page->compatible_mode){
        h=write_page->height/fontheight[write_page->font];
        if (fontwidth[write_page->font]){
            w=write_page->width/fontwidth[write_page->font];
            }else{
            w=write_page->width;
        }
        }else{
        h=write_page->height;
        w=write_page->width;
    }
    
    //PRE-ERROR CHECKING
    if (passed&1){
        if (row<write_page->top_row) goto error;
        if ((row!=h)&&(row>write_page->bottom_row)){
            if (width8050switch){//note: can assume WIDTH 80,25 as no SCREEN or WIDTH statements have been called
                width8050switch=0;
                if (row<=50){
                    if (passed&2){
                        if (column<1) goto error;
                        if (column>w) goto error;
                    }
                    std::vector<char> buffer(80*25*2);
                    uint32 c,c2;
                    c=write_page->color; c2=write_page->background_color;
                    memcpy(buffer.data(),&cmem[0xB8000],80*25*2);
                    qbsub_width(0,80,50,0,0,3);
                    memcpy(&cmem[0xB8000],buffer.data(),80*25*2);
                    write_page->color=c; write_page->background_color=c2;
                    goto width8050switch_done;
                }
            }
            goto error;
        }
    }
    width8050switch_done:
    if (passed&2){
        if (column<1) goto error;
        if (column>w) goto error;
    }
    if (passed&4){
        if (cursor<0) goto error;
        if (cursor>1) goto error;
    }
    if (passed&8){
        if (start<0) goto error;
        if (start>31) goto error;
    }
    if (passed&16){
        if (stop<0) goto error;
        if (stop>31) goto error;
    }
    
    if (passed&1) {write_page->cursor_y=row; write_page->holding_cursor=0;}
    if (passed&2) {write_page->cursor_x=column; write_page->holding_cursor=0;}
    if ((passed&3)==0){
        if (write_page->holding_cursor) write_page->holding_cursor=2;//special case[forum/index.php?topic=5255.0]
    }
    
    
    if (passed&4){
        if (cursor) cursor=1;
        write_page->cursor_show=cursor;
        if (write_page->flags&IMG_SCREEN){//page-linked attribute
            for (i=0;i<pages;i++){
                if (page[i]) img[i].cursor_show=cursor;
            }
        }//IMG_SCREEN
    }//passed&4
    
    
    if (passed&8){
        write_page->cursor_firstvalue=start;
        }else{
        start=write_page->cursor_firstvalue;
    }
    if (passed&16){
        write_page->cursor_lastvalue=stop;
        }else{
        stop=write_page->cursor_lastvalue;
    }
    if (passed&(8+16)){
        if (write_page->flags&IMG_SCREEN){//page-linked attribute
            for (i=0;i<pages;i++){
                if (page[i]){
                    img[i].cursor_firstvalue=start;
                    img[i].cursor_lastvalue=stop;
                }
            }//i
        }//IMG_SCREEN
    }
    
    return;
    
    error:
    error(5);
    return;
}
