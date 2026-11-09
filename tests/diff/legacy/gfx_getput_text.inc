// Verbatim copy of legacy internal/c/libqb.cpp (oracle commit 545b859), lines 15971-16512, 11392-11576, 9023-9041.
// Test-only reference oracle for tests/diff/diff_gfx.cpp; production code is crates/qb-runtime/src/gfx/{getput,text}.rs.
void sub_pcopy(int32 src,int32 dst){
    if (new_error) return;
    static int32 src_index,dst_index;
    static img_struct *s,*d;
    s=resolve_page_or_image_ptr(src,src_index); if (!s) goto error;
    d=resolve_page_or_image_ptr(dst,dst_index); if (!d) goto error;
    if (s==d) return;
    if (s->bytes_per_pixel!=d->bytes_per_pixel) goto error;
    if ((s->height!=d->height)||(s->width!=d->width)) goto error;
    if (s->bytes_per_pixel==1){
        if (d->mask<s->mask) goto error;//cannot copy onto a palette image with less colors
    }
    memcpy(d->offset,s->offset,d->width*d->height*d->bytes_per_pixel);
    return;
    error:
    
    error(5);
    return;
}
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
    
    
    //default fonts
    if (im->font==8) cp=&charset8x8[character][0][0];
    if (im->font==14) cp=&charset8x16[character][1][0];
    if (im->font==16) cp=&charset8x16[character][0][0];
    switch(im->print_mode){
        case 3:
        for (y2=0;y2<h;y2++){ for (x2=0;x2<8;x2++){
            if (*cp++) pset(x+x2,y+y2,color); else pset(x+x2,y+y2,background_color);
        }}
        break;
        case 1:
        for (y2=0;y2<h;y2++){ for (x2=0;x2<8;x2++){
            if (*cp++) pset(x+x2,y+y2,color);
        }}
        break;
        case 2:
        for (y2=0;y2<h;y2++){ for (x2=0;x2<8;x2++){
            if (!(*cp++)) pset(x+x2,y+y2,background_color);
        }}
        break;
        default:
        break;
    }//z
    return;
    
}//printchr
    void sub_graphics_get(float x1f,float y1f,float x2f,float y2f,void *element,uint32 mask,int32 passed){
        //"[{STEP}](?,?)-[{STEP}](?,?),?[,?]"
        //   &1            &2            &4
        if (new_error) return;
        
        static int32 x1,y1,x2,y2,z,w,h,bits,x,y,bytes,sx,sy,x3,y3,z2;
        static uint32 col,off,col1,col2,col3,col4,byte;
        
        if (read_page->text){error(5); return;}
        
        //change coordinates according to step
        if (passed&1){x1f=read_page->x+x1f; y1f=read_page->y+y1f;}
        read_page->x=x1f; read_page->y=y1f;
        if (passed&2){x2f=read_page->x+x2f; y2f=read_page->y+y2f;}
        read_page->x=x2f; read_page->y=y2f;
        
        //resolve coordinates
        if (read_page->clipping_or_scaling){
            if (read_page->clipping_or_scaling==2){
                x1=qbr_float_to_long(x1f*read_page->scaling_x+read_page->scaling_offset_x)+read_page->view_offset_x;
                y1=qbr_float_to_long(y1f*read_page->scaling_y+read_page->scaling_offset_y)+read_page->view_offset_y;
                x2=qbr_float_to_long(x2f*read_page->scaling_x+read_page->scaling_offset_x)+read_page->view_offset_x;
                y2=qbr_float_to_long(y2f*read_page->scaling_y+read_page->scaling_offset_y)+read_page->view_offset_y;
                }else{
                x1=qbr_float_to_long(x1f)+read_page->view_offset_x; y1=qbr_float_to_long(y1f)+read_page->view_offset_y;
                x2=qbr_float_to_long(x2f)+read_page->view_offset_x; y2=qbr_float_to_long(y2f)+read_page->view_offset_y;
            }
            }else{
            x1=qbr_float_to_long(x1f); y1=qbr_float_to_long(y1f);
            x2=qbr_float_to_long(x2f); y2=qbr_float_to_long(y2f);
        }
        
        //swap coordinates if reversed
        if (x2<x1){z=x1; x1=x2; x2=z;}
        if (y2<y1){z=y1; y1=y2; y2=z;}
        
        sx=read_page->width; sy=read_page->height;
        
        //boundry checking (if no mask colour was passed)
        if (!(passed&4)){
            if ((x1<0)||(y1<0)||(x2>=sx)||(y2>=sy)){error(5); return;}
        }
        
        static byte_element_struct *ele;
        ele=(byte_element_struct*)element;
        static uint16 *dimensions;
        dimensions=(uint16*)(ele->offset);
        static uint8 *cp,*cp1,*cp2,*cp3,*cp4;
        cp=(uint8*)(ele->offset+4);
        static uint32 *lp;
        lp=(uint32*)(ele->offset+4);
        
        w=x2-x1+1; h=y2-y1+1;
        bits=read_page->bits_per_pixel;
        
        if (bits==1){
            mask&=1;
            z=(w+7)>>3;
            bytes=z*h+4;
            if (bytes>ele->length){error(5); return;}
            dimensions[0]=w; dimensions[1]=h;
            for (y=y1;y<=y2;y++){
                z2=128;
                col2=0;
                off=y*sx+x1;
                for (x=x1;x<=x2;x++){
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)) col=read_page->offset[off]; else col=mask;
                    if (col) col2|=z2;
                    z2>>=1; if (!z2){z2=128; *cp++=col2; col2=0;}
                    off++;
                }
                if (z2!=128) *cp++=col2;
            }
            return;
        }//1
        
        if (bits==2){
            mask&=3;
            z=(w+7)>>3;
            bytes=z*h+4;
            if (bytes>ele->length){error(5); return;}
            dimensions[0]=w*2; dimensions[1]=h;
            for (y=y1;y<=y2;y++){
                byte=0;
                x3=0;
                off=y*sx+x1;
                for (x=x1;x<=x2;x++){
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)) col=read_page->offset[off]; else col=mask;
                    byte<<=2;
                    byte|=col;
                    if ((x3&3)==3){*cp++=byte; byte=0;}
                    x3++;
                    off++;
                }
                if (x3&3) *cp++=col2;
            }
            return;
        }//2
        
        if (bits==4){
            mask&=15;
            z=(w+7)>>3;
            bytes=z*4*h+4;
            if (bytes>ele->length){error(5); return;}
            dimensions[0]=w; dimensions[1]=h;
            y3=0;
            for (y=y1;y<=y2;y++){
                z2=128;
                off=y*sx+x1;
                cp1=cp+y3*z*4;
                cp2=cp+y3*z*4+z;
                cp3=cp+y3*z*4+z*2;
                cp4=cp+y3*z*4+z*3;
                col1=0; col2=0; col3=0; col4=0;
                for (x=x1;x<=x2;x++){
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)) col=read_page->offset[off]; else col=mask;
                    if (col&1) col1|=z2;
                    if (col&2) col2|=z2;
                    if (col&4) col3|=z2;
                    if (col&8) col4|=z2;
                    z2>>=1; if (!z2){z2=128; *cp1++=col1; *cp2++=col2; *cp3++=col3; *cp4++=col4; col1=0; col2=0; col3=0; col4=0;}
                    off++;
                }
                if (z2!=128){*cp1=col1; *cp2=col2; *cp3=col3; *cp4=col4;}
                y3++;
            }
            return;
        }//4
        
        if (bits==8){
            mask&=255;
            bytes=w*h+4;
            if (bytes>ele->length){error(5); return;}
            dimensions[0]=w*8; dimensions[1]=h;
            for (y=y1;y<=y2;y++){
                off=y*sx+x1;
                for (x=x1;x<=x2;x++){
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)) col=read_page->offset[off]; else col=mask;
                    *cp++=col;
                    off++;
                }}
                return;
        }//8
        
        if (bits==32){
            bytes=w*h*4+4;
            if (bytes>ele->length){error(5); return;}
            dimensions[0]=w; dimensions[1]=h;//note: width is left unmultiplied
            for (y=y1;y<=y2;y++){
                off=y*sx+x1;
                for (x=x1;x<=x2;x++){
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)) col=read_page->offset32[off]; else col=mask;
                    *lp++=col;
                    off++;
                }}
                return;
        }//32
        
    }//sub_graphics_get
    
    
    
    void sub_graphics_put(float x1f,float y1f,void *element,int32 option,uint32 mask,int32 passed){
        //"[{STEP}](?,?),?[,[{_CLIP}][{PSET|PRESET|AND|OR|XOR}][,?]]"
        //step->passed&1
        //clip->passed&2
        //mask->passed&4
        
        if (new_error) return;
        
        static int32 step,clip;
        step=0;
        clip=0;
        if (passed&1){step=1; passed-=1;}
        if (passed&2){clip=1; passed-=2;}
        
        static int32 x1,y1,x2,y2,z,w,h,bits,x,y,bytes,sx,sy,x3,y3,z2;
        static uint32 col,off,col1,col2,col3,col4,byte,pixelmask;
        
        if (write_page->text){error(5); return;}
        
        //change coordinates according to step
        if (step){
            x1f+=write_page->x; y1f+=write_page->y;
            write_page->x=x1f; write_page->y=y1f;
        }
        
        //resolve coordinates
        if (write_page->clipping_or_scaling){
            if (write_page->clipping_or_scaling==2){
                x1=qbr_float_to_long(x1f*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
                y1=qbr_float_to_long(y1f*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
                }else{
                x1=qbr_float_to_long(x1f)+write_page->view_offset_x; y1=qbr_float_to_long(y1f)+write_page->view_offset_y;
            }
            }else{
            x1=qbr_float_to_long(x1f); y1=qbr_float_to_long(y1f);
        }
        
        sx=write_page->width; sy=write_page->height;
        bits=write_page->bits_per_pixel;
        
        static byte_element_struct *ele;
        ele=(byte_element_struct*)element;
        static uint16 *dimensions;
        dimensions=(uint16*)(ele->offset);
        static uint8 *cp,*cp1,*cp2,*cp3,*cp4;
        cp=(uint8*)(ele->offset+4);
        static uint32 *lp;
        lp=(uint32*)(ele->offset+4);
        
        static uint8 *offp;
        static uint32 *off32p;
        
        if (4>ele->length){error(5); return;}
        
        //get dimensions
        w=dimensions[0]; h=dimensions[1];
        z=w;//(used below)
        if (bits==2){if (w&1){error(5); return;} else w>>=1;}
        if (bits==8){if (w&7){error(5); return;} else w>>=3;}
        x2=x1+w-1; y2=y1+h-1;
        
        //boundry checking (if CLIP option was not used)
        if (!clip){
            if ((x1<0)||(y1<0)||(x2>=sx)||(y2>=sy)){error(5); return;}
        }
        
        //array size check (avoid reading unacclocated memory)
        if (bits==32) z*=32;
        z=(z+7)>>3;//bits per row->bytes per row
        bytes=h*z;
        if (bits==4) bytes*=4;
        if ((bytes+4)>ele->length){error(5); return;}
        
        pixelmask=write_page->mask;
        
        if (bits==1){
            mask&=1;
            y3=0;
            for (y=y1;y<=y2;y++){
                offp=(uint8*)write_page->offset+(y*sx+x1);
                x3=0;
                for (x=x1;x<=x2;x++){
                    if (!(x3--)){x3=7; col2=*cp++;}
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)){
                        col=(col2>>x3)&1;
                        if ((!passed)||(col!=mask)){
                            switch(option){
                                case 0: *offp^=col; break;
                                case 1: *offp=col; break;
                                case 2: *offp=(~col)&pixelmask; break;
                                case 3: *offp&=col; break;
                                case 4: *offp|=col; break;
                                case 5: *offp^=col; break;
                            }
                        }//mask
                    }//bounds
                    offp++;
                }
                y3++;
            }
            return;
        }//1
        
        if (bits==2){
            mask&=3;
            y3=0;
            for (y=y1;y<=y2;y++){
                offp=(uint8*)write_page->offset+(y*sx+x1);
                x3=0;
                for (x=x1;x<=x2;x++){
                    if (!(x3--)){x3=3; col2=*cp++;}
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)){
                        col=(col2>>(x3<<1))&3;
                        if ((!passed)||(col!=mask)){
                            switch(option){
                                case 0: *offp^=col; break;
                                case 1: *offp=col; break;
                                case 2: *offp=(~col)&pixelmask; break;
                                case 3: *offp&=col; break;
                                case 4: *offp|=col; break;
                                case 5: *offp^=col; break;
                            }
                        }//mask
                    }//bounds
                    offp++;
                }
                y3++;
            }
            return;
        }//2
        
        if (bits==4){
            mask&=15;
            y3=0;
            for (y=y1;y<=y2;y++){
                offp=(uint8*)write_page->offset+(y*sx+x1);
                cp1=cp+y3*z*4;
                cp2=cp+y3*z*4+z;
                cp3=cp+y3*z*4+z*2;
                
                cp4=cp+y3*z*4+z*3;
                x3=0;
                for (x=x1;x<=x2;x++){
                    if (!(x3--)){x3=7; col1=*cp1++; col2=(*cp2++)<<1; col3=(*cp3++)<<2; col4=(*cp4++)<<3;}
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)){
                        col=((col1>>x3)&1)|((col2>>x3)&2)|((col3>>x3)&4)|((col4>>x3)&8);
                        if ((!passed)||(col!=mask)){
                            switch(option){
                                case 0: *offp^=col; break;
                                case 1: *offp=col; break;
                                case 2: *offp=(~col)&pixelmask; break;
                                case 3: *offp&=col; break;
                                case 4: *offp|=col; break;
                                case 5: *offp^=col; break;
                            }
                        }//mask
                    }//bounds
                    offp++;
                }
                y3++;
            }
            return;
        }//4
        
        if (bits==8){
            mask&=255;
            for (y=y1;y<=y2;y++){
                offp=(uint8*)write_page->offset+(y*sx+x1);
                for (x=x1;x<=x2;x++){
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)){
                        col=*cp;
                        if ((!passed)||(col!=mask)){
                            switch(option){
                                case 0: *offp^=col; break;
                                case 1: *offp=col; break;
                                case 2: *offp=(~col)&pixelmask; break;
                                case 3: *offp&=col; break;
                                case 4: *offp|=col; break;
                                case 5: *offp^=col; break;
                            }
                        }//mask
                    }//bounds
                    offp++;
                    cp++;
                }}
                return;
        }//8
        
        if (bits==32){
            for (y=y1;y<=y2;y++){
                off32p=(uint32*)write_page->offset32+(y*sx+x1);
                for (x=x1;x<=x2;x++){
                    if ((x>=0)&&(y>=0)&&(x<sx)&&(y<sy)){
                        col=*lp;
                        if ((!passed)||(col!=mask)){
                            switch(option){
                                case 0: *off32p^=col; break;
                                case 1: *off32p=col; break;
                                case 2: *off32p=(~col)&pixelmask; break;
                                case 3: *off32p&=col; break;
                                case 4: *off32p|=col; break;
                                case 5: *off32p^=col; break;
                            }
                        }//mask
                    }//bounds
                    off32p++;
                    lp++;
                }}
                return;
        }//32
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        /*
            static byte_element_struct *ele;
            ele=(byte_element_struct*)element;
            static int32 x,y;
            static int32 sx,sy,c,px,py;
            static uint8 *cp;
            static int32 *lp;
            
            sx=((uint16*)ele->offset)[0];
            sy=((uint16*)ele->offset)[1];
            cp=(uint8*)(ele->offset+4);
            lp=(int32*)cp;
            
            
            static int32 sizeinbytes;
            static int32 byte;
            static int32 bitvalue;
            static int32 bytesperrow;
            static int32 row2offset;
            static int32 row3offset;
            static int32 row4offset;
            
            static int32 longval;
            
            if (write_page->bits_per_pixel==8){
            mask&=255;
            //create error if not divisible by 8!
            sx>>=3;
            }
            
            if (write_page->bits_per_pixel==1){
            mask&=1;
            }
            
            if (write_page->bits_per_pixel==2){
            mask&=3;
            sx>>=1;
            }
            
            
            if (write_page->bits_per_pixel==4){
            mask&=15;
            bytesperrow=sx>>3; if (sx&7) bytesperrow++;
            row2offset=bytesperrow;
            row3offset=bytesperrow*2;
            row4offset=bytesperrow*3;
            }
            
            
            for (y=0;y<sy;y++){
            py=y1+y;
            
            if (write_page->bits_per_pixel==4){
            bitvalue=128;
            byte=0;
            }
            
            for (x=0;x<sx;x++){
            px=x1+x;
            
            
            //get colour
            if (write_page->bits_per_pixel==8){
            c=*cp;
            cp++;
            }
            
            if (write_page->bits_per_pixel==4){
            byte=x>>3;
            c=0;
            if (cp[byte]&bitvalue) c|=1;
            if (cp[row2offset+byte]&bitvalue) c|=2;
            if (cp[row3offset+byte]&bitvalue) c|=4;
            if (cp[row4offset+byte]&bitvalue) c|=8;
            bitvalue>>=1; if (bitvalue==0) bitvalue=128;
            }
            
            
            if (write_page->bits_per_pixel==1){
            if (!(x&7)){
            byte=*cp;
            cp++;
            }
            c=(byte&128)>>7; byte<<=1;
            }
            
            if (write_page->bits_per_pixel==2){
            if (!(x&3)){
            byte=*cp;
            cp++;
            }
            c=(byte&192)>>6; byte<<=2;
            }
            
            
            if ((px>=0)&&(px<write_page->width)&&(py>=0)&&(py<write_page->height)){
            
            //check color
            if (passed){
            if (c==mask) goto maskpixel;
            }
            
            
            //"pset" color
            
            //PUT[{STEP}](?,?),?[,[{PSET|PRESET|AND|OR|XOR}][,[?]]]
            //apply option
            
            if (option==1){
            write_page->offset[py*write_page->width+px]=c;
            }
            if (option==2){
            //PRESET=bitwise NOT
            write_page->offset[py*write_page->width+px]=(~c)&write_page->mask;
            }
            if (option==3){
            write_page->offset[py*write_page->width+px]&=c;
            }
            if (option==4){
            write_page->offset[py*write_page->width+px]|=c;
            }
            if ((option==5)||(option==0)){
            write_page->offset[py*write_page->width+px]^=c;
            }
            
            
            
            
            
            
            }
            maskpixel:;
            
            
            }//x
            
            
            if (write_page->bits_per_pixel==4) cp+=(bytesperrow*4);
            
            //if (_bits_per_pixel==1){
            // if (sx&7) cp++;
            //}
            
            //if (_bits_per_pixel==2){
            // if (sx&3) cp++;
            //}
            
            
            }//y
        */
        
    }
