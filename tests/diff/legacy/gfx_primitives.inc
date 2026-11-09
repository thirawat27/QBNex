// Verbatim copy of legacy internal/c/libqb.cpp (commit 4aa6f1f; this code is unchanged from the oracle 545b859), lines 9285-11091.
// Test-only reference oracle for tests/diff/diff_gfx.cpp; production code is crates/qb-runtime/src/gfx.
void pset_and_clip(int32 x,int32 y,uint32 col){
    
    if ((x>=write_page->view_x1)&&(x<=write_page->view_x2)&&(y>=write_page->view_y1)&&(y<=write_page->view_y2)){
        
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
        
    }//within viewport
    return;
}

void qb32_boxfill(float x1f,float y1f,float x2f,float y2f,uint32 col){
    static int32 x1,y1,x2,y2,i,width,img_width,x,y,d_width,a,a2,v1,v2,v3;
    static uint8 *p,*cp,*cp2,*cp3;
    static uint32 *lp,*lp_last,*lp_first;
    static uint32 *doff32,destcol;
    
    //resolve coordinates
    if (write_page->clipping_or_scaling){
        if (write_page->clipping_or_scaling==2){
            x1=qbr_float_to_long(x1f*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            y1=qbr_float_to_long(y1f*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            x2=qbr_float_to_long(x2f*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            y2=qbr_float_to_long(y2f*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            }else{
            x1=qbr_float_to_long(x1f)+write_page->view_offset_x; y1=qbr_float_to_long(y1f)+write_page->view_offset_y;
            x2=qbr_float_to_long(x2f)+write_page->view_offset_x; y2=qbr_float_to_long(y2f)+write_page->view_offset_y;
        }
        }else{
        x1=qbr_float_to_long(x1f); y1=qbr_float_to_long(y1f);
        x2=qbr_float_to_long(x2f); y2=qbr_float_to_long(y2f);
    }
    
    //swap coordinates (if necessary)
    if (x1>x2){i=x1; x1=x2; x2=i;}
    if (y1>y2){i=y1; y1=y2; y2=i;}
    
    //exit without rendering if necessary
    if (x2<write_page->view_x1) return;
    if (x1>write_page->view_x2) return;
    if (y2<write_page->view_y1) return;
    if (y1>write_page->view_y2) return;
    
    //crop coordinates
    if (x1<write_page->view_x1) x1=write_page->view_x1;
    if (y1<write_page->view_y1) y1=write_page->view_y1;
    if (x1>write_page->view_x2) x1=write_page->view_x2;
    if (y1>write_page->view_y2) y1=write_page->view_y2;
    if (x2<write_page->view_x1) x2=write_page->view_x1;
    if (y2<write_page->view_y1) y2=write_page->view_y1;
    if (x2>write_page->view_x2) x2=write_page->view_x2;
    if (y2>write_page->view_y2) y2=write_page->view_y2;
    
    if (write_page->bytes_per_pixel==1){
        col&=write_page->mask;
        width=x2-x1+1;
        img_width=write_page->width;
        p=write_page->offset+y1*write_page->width+x1;
        i=y2-y1+1;
        loop:
        memset(p,col,width);
        p+=img_width;
        if (--i) goto loop;
        return;
    }//1
    
    //assume 32-bit
    //optomized
    //alpha disabled or full alpha?
    a=col>>24;
    if ((write_page->alpha_disabled)||(a==255)){
        width=x2-x1+1;
        y=y2-y1+1;
        img_width=write_page->width;
        //build first line pixel by pixel
        lp_first=write_page->offset32+y1*img_width+x1;
        lp=lp_first-1; lp_last=lp+width;
        while (lp++<lp_last) *lp=col;
        //copy remaining lines
        lp=lp_first;
        width*=4;
        while(y--){
            memcpy(lp,lp_first,width);
            lp+=img_width;
        }
        return;
    }
    //no alpha?
    if (!a) return;
    //half alpha?
    img_width=write_page->width;
    doff32=write_page->offset32+y1*img_width+x1;
    width=x2-x1+1;
    d_width=img_width-width;
    if (a==128){
        col&=0xFEFEFE;
        y=y2-y1+1;
        while(y--){
            x=width;
            while(x--){
                *doff32++=(((*doff32&0xFEFEFE)+col)>>1)+(ablend128[*doff32>>24]<<24);
            }
            doff32+=d_width;
        }
        return;
    }
    if (a==127){
        col&=0xFEFEFE;
        y=y2-y1+1;
        while(y--){
            x=width;
            while(x--){
                *doff32++=(((*doff32&0xFEFEFE)+col)>>1)+(ablend127[*doff32>>24]<<24);
            }
            doff32+=d_width;
        }
        return;
    }
    //ranged alpha
    cp=cblend+(a<<16);
    a2=a<<8;
    cp3=cp+(col>>8&0xFF00);
    cp2=cp+(col&0xFF00);
    cp+=(col<<8&0xFF00);
    y=y2-y1+1;
    while(y--){
        x=width;
        while(x--){
            destcol=*doff32;
            *doff32++=
            cp[destcol&255]
            +(cp2[destcol>>8&255]<<8)
            +(cp3[destcol>>16&255]<<16)
            +(ablend[(destcol>>24)+a2]<<24);
        }
        doff32+=d_width;
    }
    return;
}


void fast_boxfill(int32 x1,int32 y1,int32 x2,int32 y2,uint32 col){
    //assumes:
    //actual coordinates passed
    //left->right, top->bottom order
    //on-screen
    static int32 i,width,img_width,x,y,d_width,a,a2,v1,v2,v3;
    static uint8 *p,*cp,*cp2,*cp3;
    static uint32 *lp,*lp_last,*lp_first;
    static uint32 *doff32,destcol;
    
    if (write_page->bytes_per_pixel==1){
        col&=write_page->mask;
        width=x2-x1+1;
        img_width=write_page->width;
        p=write_page->offset+y1*write_page->width+x1;
        i=y2-y1+1;
        loop:
        memset(p,col,width);
        p+=img_width;
        if (--i) goto loop;
        return;
    }//1
    
    //assume 32-bit
    //optomized
    //alpha disabled or full alpha?
    a=col>>24;
    if ((write_page->alpha_disabled)||(a==255)){
        
        width=x2-x1+1;
        y=y2-y1+1;
        img_width=write_page->width;
        //build first line pixel by pixel
        lp_first=write_page->offset32+y1*img_width+x1;
        lp=lp_first-1; lp_last=lp+width;
        while (lp++<lp_last) *lp=col;
        //copy remaining lines
        lp=lp_first;
        width*=4;
        while(y--){
            memcpy(lp,lp_first,width);
            lp+=img_width;
        }
        return;
    }
    //no alpha?
    if (!a) return;
    //half alpha?
    img_width=write_page->width;
    doff32=write_page->offset32+y1*img_width+x1;
    width=x2-x1+1;
    d_width=img_width-width;
    if (a==128){
        col&=0xFEFEFE;
        y=y2-y1+1;
        while(y--){
            x=width;
            while(x--){
                *doff32++=(((*doff32&0xFEFEFE)+col)>>1)+(ablend128[*doff32>>24]<<24);
            }
            doff32+=d_width;
        }
        return;
    }
    if (a==127){
        col&=0xFEFEFE;
        y=y2-y1+1;
        while(y--){
            x=width;
            while(x--){
                *doff32++=(((*doff32&0xFEFEFE)+col)>>1)+(ablend127[*doff32>>24]<<24);
            }
            doff32+=d_width;
        }
        return;
    }
    //ranged alpha
    cp=cblend+(a<<16);
    a2=a<<8;
    cp3=cp+(col>>8&0xFF00);
    cp2=cp+(col&0xFF00);
    cp+=(col<<8&0xFF00);
    y=y2-y1+1;
    while(y--){
        x=width;
        while(x--){
            destcol=*doff32;
            *doff32++=
            cp[destcol&255]
            +(cp2[destcol>>8&255]<<8)
            +(cp3[destcol>>16&255]<<16)
            +(ablend[(destcol>>24)+a2]<<24);
        }
        doff32+=d_width;
    }
    return;
}






//copied from qb32_line with the following modifications
//i. pre-WINDOW'd & VIEWPORT'd int32 co-ordinates
//ii. all references to style & lineclip_skippixels commented
//iii. declaration of x1,y1,x2,y2,x1f,y1f changed, some declarations removed
void fast_line(int32 x1,int32 y1,int32 x2,int32 y2,uint32 col){
    static int32 l,l2,mi;
    static float m,x1f,y1f;
    
    lineclip(x1,y1,x2,y2,write_page->view_x1,write_page->view_y1,write_page->view_x2,write_page->view_y2);
    
    //style=(style&65535)+(style<<16);
    //lineclip_skippixels&=15;
    //style=_lrotl(style,lineclip_skippixels);
    
    if (lineclip_draw){
        l=abs(lineclip_x1-lineclip_x2);
        l2=abs(lineclip_y1-lineclip_y2);
        if (l>l2){
            
            //x-axis distance is larger
            y1f=lineclip_y1;
            if (l){//following only applies if drawing more than one pixel
                m=((float)lineclip_y2-(float)lineclip_y1)/(float)l;
                if (lineclip_x2>=lineclip_x1) mi=1; else mi=-1;//direction of change
            }
            l++;
            while (l--){
                if (y1f<0) lineclip_y1=y1f-0.5f; else lineclip_y1=y1f+0.5f;
                
                //if ((style=_lrotl(style,1))&1){
                pset(lineclip_x1,lineclip_y1,col);
            //}
            
            lineclip_x1+=mi;
            y1f+=m;
        }
        
        }else{
        
        //y-axis distance is larger
        x1f=lineclip_x1;
        if (l2){//following only applies if drawing more than one pixel
            m=((float)lineclip_x2-(float)lineclip_x1)/(float)l2;
            if (lineclip_y2>=lineclip_y1) mi=1; else mi=-1;//direction of change
        }
        l2++;
        while (l2--){
            if (x1f<0) lineclip_x1=x1f-0.5f; else lineclip_x1=x1f+0.5f;
            //if ((style=_lrotl(style,1))&1){
            pset(lineclip_x1,lineclip_y1,col);
        //}
        lineclip_y1+=mi;
        x1f+=m;
    }
    
}

}//lineclip_draw
return;
}























void qb32_line(float x1f,float y1f,float x2f,float y2f,uint32 col,uint32 style){
    static int32 x1,y1,x2,y2,l,l2,mi;
    static float m;
    
    //resolve coordinates
    if (write_page->clipping_or_scaling){
        if (write_page->clipping_or_scaling==2){
            x1=qbr_float_to_long(x1f*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            y1=qbr_float_to_long(y1f*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            x2=qbr_float_to_long(x2f*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            y2=qbr_float_to_long(y2f*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            }else{
            x1=qbr_float_to_long(x1f)+write_page->view_offset_x; y1=qbr_float_to_long(y1f)+write_page->view_offset_y;
            x2=qbr_float_to_long(x2f)+write_page->view_offset_x; y2=qbr_float_to_long(y2f)+write_page->view_offset_y;
        }
        }else{
        x1=qbr_float_to_long(x1f); y1=qbr_float_to_long(y1f);
        x2=qbr_float_to_long(x2f); y2=qbr_float_to_long(y2f);
    }
    
    lineclip(x1,y1,x2,y2,write_page->view_x1,write_page->view_y1,write_page->view_x2,write_page->view_y2);
    
    style=(style&65535)+(style<<16);
    lineclip_skippixels&=15;
    style=rotateLeft(style,lineclip_skippixels);
    
    if (lineclip_draw){
        l=abs(lineclip_x1-lineclip_x2);
        l2=abs(lineclip_y1-lineclip_y2);
        if (l>l2){
            
            //x-axis distance is larger
            y1f=lineclip_y1;
            if (l){//following only applies if drawing more than one pixel
                m=((float)lineclip_y2-(float)lineclip_y1)/(float)l;
                if (lineclip_x2>=lineclip_x1) mi=1; else mi=-1;//direction of change
            }
            l++;
            while (l--){
                if (y1f<0) lineclip_y1=y1f-0.5f; else lineclip_y1=y1f+0.5f;
                
                if ((style=rotateLeft(style,1))&1){
                    pset(lineclip_x1,lineclip_y1,col);
                }
                
                lineclip_x1+=mi;
                y1f+=m;
            }
            
            }else{
            
            //y-axis distance is larger
            x1f=lineclip_x1;
            if (l2){//following only applies if drawing more than one pixel
                m=((float)lineclip_x2-(float)lineclip_x1)/(float)l2;
                if (lineclip_y2>=lineclip_y1) mi=1; else mi=-1;//direction of change
            }
            l2++;
            while (l2--){
                if (x1f<0) lineclip_x1=x1f-0.5f; else lineclip_x1=x1f+0.5f;
                if ((style=rotateLeft(style,1))&1){
                    pset(lineclip_x1,lineclip_y1,col);
                }
                lineclip_y1+=mi;
                x1f+=m;
            }
            
        }
        
    }//lineclip_draw
    return;
}



void sub_line(float x1,float y1,float x2,float y2,uint32 col,int32 bf,uint32 style,int32 passed){
    if (new_error) return;
    if (write_page->text){error(5); return;}
    /*
        '"[[{STEP}](?,?)]-[{STEP}](?,?)[,[?][,[{B|BF}][,?]]]"
        LINE -(10, 10) 'flags: 0
        LINE (0, 0)-(10, 10) 'flags: 1
        LINE -STEP(10, 10) 'flags: 2
        LINE STEP(0, 0)-(10, 10) 'flags: 1+4
    */
    
    //adjust coordinates and qb graphics cursor position based on STEP
    if (passed&1){
        if (passed&4){x1=write_page->x+x1; y1=write_page->y+y1;}
        write_page->x=x1; write_page->y=y1;
        }else{
        x1=write_page->x; y1=write_page->y;
    }
    if (passed&2){x2=write_page->x+x2; y2=write_page->y+y2;}
    write_page->x=x2; write_page->y=y2;
    
    if (bf==0){//line
        if ((passed&16)==0) style=0xFFFF;
        if ((passed&8)==0) col=write_page->color;
        write_page->draw_color=col;
        qb32_line(x1,y1,x2,y2,col,style);
        return;
    }
    
    if (bf==1){//rectangle
        if ((passed&16)==0) style=0xFFFF;
        if ((passed&8)==0) col=write_page->color;
        write_page->draw_color=col;
        qb32_line(x1,y1,x2,y1,col,style);
        qb32_line(x2,y1,x2,y2,col,style);
        qb32_line(x2,y2,x1,y2,col,style);
        qb32_line(x1,y2,x1,y1,col,style);
        return;
    }
    
    if (bf==2){//filled box
        if ((passed&8)==0) col=write_page->color;
        write_page->draw_color=col;
        qb32_boxfill(x1,y1,x2,y2,col);
        return;
    }
    
}//sub_line

































//3 paint routines exist for color (not textured) filling
//i) 8-bit
//ii) 32-bit no-alpha
//iii) 32-bit
//simple comparisons are used, the alpha value is part of that comparison in all cases
//even if blending is disabled (a fixed color is likely to have a fixed alpha value anyway),
//and this allows for filling alpha regions

//32-bit WITH BENDING
void sub_paint32(float x,float y,uint32 fillcol,uint32 bordercol,int32 passed){
    
    //uses 2 buffers, a and b, and swaps between them for reading and creating
    static uint32 a_n=0;
    static std::vector<uint16> a_x_storage(65536),a_y_storage(65536);
    static std::vector<uint8> a_t_storage(65536);
    static uint32 b_n=0;
    static std::vector<uint16> b_x_storage(65536),b_y_storage(65536);
    static std::vector<uint8> b_t_storage(65536);
    static std::vector<uint8> done_storage(640*480,0);
    uint16 *a_x=a_x_storage.data(),*a_y=a_y_storage.data();
    uint8 *a_t=a_t_storage.data();
    uint16 *b_x=b_x_storage.data(),*b_y=b_y_storage.data();
    uint8 *b_t=b_t_storage.data();
    uint8 *done=done_storage.data();
    static int32 ix,iy,i,t,x2,y2;
    static uint32 offset;
    static uint8 *cp;
    static uint16 *sp;
    //overrides
    static int32 done_size=640*480;
    static uint32 *qbg_active_page_offset;//override
    static int32 qbg_width,qbg_view_x1,qbg_view_y1,qbg_view_x2,qbg_view_y2;//override
    static uint32 *doff32,destcol;
    
    if ((passed&2)==0) fillcol=write_page->color;
    if ((passed&4)==0) bordercol=fillcol;
    write_page->draw_color=fillcol;
    
    if (passed&1){write_page->x+=x; write_page->y+=y;}else{write_page->x=x; write_page->y=y;}
    
    if (write_page->clipping_or_scaling){
        if (write_page->clipping_or_scaling==2){
            ix=qbr_float_to_long(write_page->x*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            iy=qbr_float_to_long(write_page->y*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            }else{
            ix=qbr_float_to_long(write_page->x)+write_page->view_offset_x; iy=qbr_float_to_long(write_page->y)+write_page->view_offset_y;
        }
        }else{
        ix=qbr_float_to_long(write_page->x); iy=qbr_float_to_long(write_page->y);
    }
    
    //return if offscreen
    if ((ix<write_page->view_x1)||(iy<write_page->view_y1)||(ix>write_page->view_x2)||(iy>write_page->view_y2)){
        return;
    }
    
    //overrides
    qbg_active_page_offset=write_page->offset32;
    qbg_width=write_page->width;
    qbg_view_x1=write_page->view_x1;
    qbg_view_y1=write_page->view_y1;
    qbg_view_x2=write_page->view_x2;
    qbg_view_y2=write_page->view_y2;
    i=write_page->width*write_page->height;
    if (i>done_size){
        done_storage.assign(i,0);
        done_size=i;
        done=done_storage.data();
    }
    
    //return if first point is the bordercolor
    if (qbg_active_page_offset[iy*qbg_width+ix]==bordercol) return;
    
    //create first node
    a_x[0]=ix; a_y[0]=iy;
    a_t[0]=15;
    //types:
    //&1=check left
    //&2=check right
    //&4=check above
    //&8=check below
    
    
    a_n=1;
    //qbg_active_page_offset[iy*qbg_width+ix]=fillcol;
    offset=iy*qbg_width+ix;
    //--------plot pixel--------
    doff32=qbg_active_page_offset+offset;
    switch(fillcol&0xFF000000){
        case 0xFF000000:
        *doff32=fillcol;
        break;
        case 0x0:
        doff32;
        break;
        case 0x80000000:
        *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
        break; 
        case 0x7F000000:
        *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
        break;
        default:
        destcol=*doff32;
        cp=cblend+(fillcol>>24<<16);
        *doff32=
        cp[(fillcol<<8&0xFF00)+(destcol&255)    ]
        +(cp[(fillcol&0xFF00)   +(destcol>>8&255) ]<<8)
        +(cp[(fillcol>>8&0xFF00)+(destcol>>16&255)]<<16)
        +(ablend[(fillcol>>24)+(destcol>>16&0xFF00)]<<24);
    };//switch
    //--------done plot pixel--------
    done[iy*qbg_width+ix]=1;
    
    nextpass:
    b_n=0;
    for (i=0;i<a_n;i++){
        t=a_t[i]; ix=a_x[i]; iy=a_y[i];
        
        //left
        if (t&1){
            x2=ix-1; y2=iy;
            if (x2>=qbg_view_x1){
                offset=y2*qbg_width+x2;
                if (!done[offset]){
                    done[offset]=1;
                    if (qbg_active_page_offset[offset]!=bordercol){
                        //--------plot pixel--------
                        doff32=qbg_active_page_offset+offset;
                        switch(fillcol&0xFF000000){
                            case 0xFF000000:
                            *doff32=fillcol;
                            break;
                            case 0x0:
                            doff32;
                            break;
                            case 0x80000000:
                            *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
                            break; 
                            case 0x7F000000:
                            *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
                            break;
                            default:
                            destcol=*doff32;
                            cp=cblend+(fillcol>>24<<16);
                            *doff32=
                            cp[(fillcol<<8&0xFF00)+(destcol&255)    ]
                            +(cp[(fillcol&0xFF00)   +(destcol>>8&255) ]<<8)
                            +(cp[(fillcol>>8&0xFF00)+(destcol>>16&255)]<<16)
                            +(ablend[(fillcol>>24)+(destcol>>16&0xFF00)]<<24);
                        };//switch
                        //--------done plot pixel--------
                        b_t[b_n]=13; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                    }}}}
                    
                    //right
                    if (t&2){
                        x2=ix+1; y2=iy;
                        if (x2<=qbg_view_x2){
                            offset=y2*qbg_width+x2;
                            if (!done[offset]){
                                done[offset]=1;
                                if (qbg_active_page_offset[offset]!=bordercol){
                                    //--------plot pixel--------
                                    doff32=qbg_active_page_offset+offset;
                                    switch(fillcol&0xFF000000){
                                        case 0xFF000000:
                                        *doff32=fillcol;
                                        break;
                                        case 0x0:
                                        doff32;
                                        break;
                                        case 0x80000000:
                                        *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
                                        break; 
                                        case 0x7F000000:
                                        *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
                                        break;
                                        default:
                                        destcol=*doff32;
                                        cp=cblend+(fillcol>>24<<16);
                                        *doff32=
                                        cp[(fillcol<<8&0xFF00)+(destcol&255)    ]
                                        +(cp[(fillcol&0xFF00)   +(destcol>>8&255) ]<<8)
                                        +(cp[(fillcol>>8&0xFF00)+(destcol>>16&255)]<<16)
                                        +(ablend[(fillcol>>24)+(destcol>>16&0xFF00)]<<24);
                                    };//switch
                                    //--------done plot pixel--------
                                    b_t[b_n]=14; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                }}}}
                                
                                //above
                                if (t&4){
                                    x2=ix; y2=iy-1;
                                    if (y2>=qbg_view_y1){
                                        offset=y2*qbg_width+x2;
                                        if (!done[offset]){
                                            done[offset]=1;
                                            if (qbg_active_page_offset[offset]!=bordercol){
                                                //--------plot pixel--------
                                                doff32=qbg_active_page_offset+offset;
                                                switch(fillcol&0xFF000000){
                                                    case 0xFF000000:
                                                    *doff32=fillcol;
                                                    break;
                                                    case 0x0:
                                                    doff32;
                                                    break;
                                                    case 0x80000000:
                                                    *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
                                                    break; 
                                                    case 0x7F000000:
                                                    *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
                                                    break;
                                                    default:
                                                    destcol=*doff32;
                                                    cp=cblend+(fillcol>>24<<16);
                                                    *doff32=
                                                    cp[(fillcol<<8&0xFF00)+(destcol&255)    ]
                                                    +(cp[(fillcol&0xFF00)   +(destcol>>8&255) ]<<8)
                                                    +(cp[(fillcol>>8&0xFF00)+(destcol>>16&255)]<<16)
                                                    +(ablend[(fillcol>>24)+(destcol>>16&0xFF00)]<<24);
                                                };//switch
                                                //--------done plot pixel--------
                                                b_t[b_n]=7; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                            }}}}
                                            
                                            //below
                                            if (t&8){
                                                x2=ix; y2=iy+1;
                                                if (y2<=qbg_view_y2){
                                                    offset=y2*qbg_width+x2;
                                                    if (!done[offset]){
                                                        done[offset]=1;
                                                        if (qbg_active_page_offset[offset]!=bordercol){
                                                            //--------plot pixel--------
                                                            doff32=qbg_active_page_offset+offset;
                                                            switch(fillcol&0xFF000000){
                                                                case 0xFF000000:
                                                                *doff32=fillcol;
                                                                break;
                                                                case 0x0:
                                                                doff32;
                                                                break;
                                                                case 0x80000000:
                                                                *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
                                                                break; 
                                                                case 0x7F000000:
                                                                *doff32=(((*doff32&0xFEFEFE)+(fillcol&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
                                                                break;
                                                                default:
                                                                destcol=*doff32;
                                                                cp=cblend+(fillcol>>24<<16);
                                                                *doff32=
                                                                cp[(fillcol<<8&0xFF00)+(destcol&255)    ]
                                                                +(cp[(fillcol&0xFF00)   +(destcol>>8&255) ]<<8)
                                                                +(cp[(fillcol>>8&0xFF00)+(destcol>>16&255)]<<16)
                                                                +(ablend[(fillcol>>24)+(destcol>>16&0xFF00)]<<24);
                                                            };//switch
                                                            //--------done plot pixel--------
                                                            b_t[b_n]=11; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                                        }}}}
                                                        
    }//i
    
    //no new nodes?
    if (b_n==0){
        memset(done,0,write_page->width*write_page->height);//cleanup
        return;//finished!
    }
    
    //swap a & b arrays
    sp=a_x; a_x=b_x; b_x=sp;
    sp=a_y; a_y=b_y; b_y=sp;
    cp=a_t; a_t=b_t; b_t=cp;
    a_n=b_n;
    
    goto nextpass;
}


//32-bit NO ALPHA BENDING
void sub_paint32x(float x,float y,uint32 fillcol,uint32 bordercol,int32 passed){
    
    //uses 2 buffers, a and b, and swaps between them for reading and creating
    static uint32 a_n=0;
    static std::vector<uint16> a_x_storage(65536),a_y_storage(65536);
    static std::vector<uint8> a_t_storage(65536);
    static uint32 b_n=0;
    static std::vector<uint16> b_x_storage(65536),b_y_storage(65536);
    static std::vector<uint8> b_t_storage(65536);
    static std::vector<uint8> done_storage(640*480,0);
    uint16 *a_x=a_x_storage.data(),*a_y=a_y_storage.data();
    uint8 *a_t=a_t_storage.data();
    uint16 *b_x=b_x_storage.data(),*b_y=b_y_storage.data();
    uint8 *b_t=b_t_storage.data();
    uint8 *done=done_storage.data();
    static int32 ix,iy,i,t,x2,y2;
    static uint32 offset;
    static uint8 *cp;
    static uint16 *sp;
    //overrides
    static int32 done_size=640*480;
    static uint32 *qbg_active_page_offset;//override
    static int32 qbg_width,qbg_view_x1,qbg_view_y1,qbg_view_x2,qbg_view_y2;//override
    
    if ((passed&2)==0) fillcol=write_page->color;
    if ((passed&4)==0) bordercol=fillcol;
    write_page->draw_color=fillcol;
    
    if (passed&1){write_page->x+=x; write_page->y+=y;}else{write_page->x=x; write_page->y=y;}
    
    if (write_page->clipping_or_scaling){
        if (write_page->clipping_or_scaling==2){
            ix=qbr_float_to_long(write_page->x*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            iy=qbr_float_to_long(write_page->y*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            }else{
            ix=qbr_float_to_long(write_page->x)+write_page->view_offset_x; iy=qbr_float_to_long(write_page->y)+write_page->view_offset_y;
        }
        }else{
        ix=qbr_float_to_long(write_page->x); iy=qbr_float_to_long(write_page->y);
    }
    
    //return if offscreen
    if ((ix<write_page->view_x1)||(iy<write_page->view_y1)||(ix>write_page->view_x2)||(iy>write_page->view_y2)){
        return;
    }
    
    //overrides
    qbg_active_page_offset=write_page->offset32;
    qbg_width=write_page->width;
    qbg_view_x1=write_page->view_x1;
    qbg_view_y1=write_page->view_y1;
    qbg_view_x2=write_page->view_x2;
    qbg_view_y2=write_page->view_y2;
    i=write_page->width*write_page->height;
    if (i>done_size){
        done_storage.assign(i,0);
        done_size=i;
        done=done_storage.data();
    }
    
    //return if first point is the bordercolor
    if (qbg_active_page_offset[iy*qbg_width+ix]==bordercol) return;
    
    //create first node
    a_x[0]=ix; a_y[0]=iy;
    a_t[0]=15;
    //types:
    //&1=check left
    //&2=check right
    //&4=check above
    //&8=check below
    
    a_n=1;
    qbg_active_page_offset[iy*qbg_width+ix]=fillcol;
    done[iy*qbg_width+ix]=1;
    
    nextpass:
    b_n=0;
    for (i=0;i<a_n;i++){
        t=a_t[i]; ix=a_x[i]; iy=a_y[i];
        
        //left
        if (t&1){
            x2=ix-1; y2=iy;
            if (x2>=qbg_view_x1){
                offset=y2*qbg_width+x2;
                if (!done[offset]){
                    done[offset]=1;
                    if (qbg_active_page_offset[offset]!=bordercol){
                        qbg_active_page_offset[offset]=fillcol;
                        b_t[b_n]=13; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                    }}}}
                    
                    //right
                    if (t&2){
                        x2=ix+1; y2=iy;
                        if (x2<=qbg_view_x2){
                            offset=y2*qbg_width+x2;
                            if (!done[offset]){
                                done[offset]=1;
                                if (qbg_active_page_offset[offset]!=bordercol){
                                    qbg_active_page_offset[offset]=fillcol;
                                    b_t[b_n]=14; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                }}}}
                                
                                //above
                                if (t&4){
                                    x2=ix; y2=iy-1;
                                    if (y2>=qbg_view_y1){
                                        offset=y2*qbg_width+x2;
                                        if (!done[offset]){
                                            done[offset]=1;
                                            if (qbg_active_page_offset[offset]!=bordercol){
                                                qbg_active_page_offset[offset]=fillcol;
                                                b_t[b_n]=7; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                            }}}}
                                            
                                            //below
                                            if (t&8){
                                                x2=ix; y2=iy+1;
                                                if (y2<=qbg_view_y2){
                                                    offset=y2*qbg_width+x2;
                                                    if (!done[offset]){
                                                        done[offset]=1;
                                                        if (qbg_active_page_offset[offset]!=bordercol){
                                                            qbg_active_page_offset[offset]=fillcol;
                                                            b_t[b_n]=11; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                                        }}}}
                                                        
    }//i
    
    //no new nodes?
    if (b_n==0){
        memset(done,0,write_page->width*write_page->height);//cleanup
        return;//finished!
    }
    
    //swap a & b arrays
    sp=a_x; a_x=b_x; b_x=sp;
    sp=a_y; a_y=b_y; b_y=sp;
    cp=a_t; a_t=b_t; b_t=cp;
    a_n=b_n;
    
    goto nextpass;
}



//8-bit (default entry point)
void sub_paint(float x,float y,uint32 fillcol,uint32 bordercol,qbs *backgroundstr,int32 passed){
    if (new_error) return;
    if (write_page->text){error(5); return;}
    if (passed&8){error(5); return;}
    
    if (write_page->bytes_per_pixel==4){
        if (write_page->alpha_disabled){
            sub_paint32x(x,y,fillcol,bordercol,passed);
            return;
            }else{
            sub_paint32(x,y,fillcol,bordercol,passed);
            return;
        }
    }
    
    //uses 2 buffers, a and b, and swaps between them for reading and creating
    static uint32 a_n=0;
    static std::vector<uint16> a_x_storage(65536),a_y_storage(65536);
    static std::vector<uint8> a_t_storage(65536);
    static uint32 b_n=0;
    static std::vector<uint16> b_x_storage(65536),b_y_storage(65536);
    static std::vector<uint8> b_t_storage(65536);
    static std::vector<uint8> done_storage(640*480,0);
    uint16 *a_x=a_x_storage.data(),*a_y=a_y_storage.data();
    uint8 *a_t=a_t_storage.data();
    uint16 *b_x=b_x_storage.data(),*b_y=b_y_storage.data();
    uint8 *b_t=b_t_storage.data();
    uint8 *done=done_storage.data();
    static int32 ix,iy,i,t,x2,y2;
    static uint32 offset;
    static uint8 *cp;
    static uint16 *sp;
    //overrides
    static int32 done_size=640*480;
    static uint8 *qbg_active_page_offset;//override
    static int32 qbg_width,qbg_view_x1,qbg_view_y1,qbg_view_x2,qbg_view_y2;//override
    
    if ((passed&2)==0) fillcol=write_page->color;
    if ((passed&4)==0) bordercol=fillcol;
    fillcol&=write_page->mask;
    bordercol&=write_page->mask;
    write_page->draw_color=fillcol;
    
    if (passed&1){write_page->x+=x; write_page->y+=y;}else{write_page->x=x; write_page->y=y;}
    
    if (write_page->clipping_or_scaling){
        if (write_page->clipping_or_scaling==2){
            ix=qbr_float_to_long(write_page->x*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            iy=qbr_float_to_long(write_page->y*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            }else{
            ix=qbr_float_to_long(write_page->x)+write_page->view_offset_x; iy=qbr_float_to_long(write_page->y)+write_page->view_offset_y;
        }
        }else{
        ix=qbr_float_to_long(write_page->x); iy=qbr_float_to_long(write_page->y);
    }
    
    //return if offscreen
    if ((ix<write_page->view_x1)||(iy<write_page->view_y1)||(ix>write_page->view_x2)||(iy>write_page->view_y2)){
        return;
    }
    
    //overrides
    qbg_active_page_offset=write_page->offset;
    qbg_width=write_page->width;
    qbg_view_x1=write_page->view_x1;
    qbg_view_y1=write_page->view_y1;
    qbg_view_x2=write_page->view_x2;
    qbg_view_y2=write_page->view_y2;
    i=write_page->width*write_page->height;
    if (i>done_size){
        done_storage.assign(i,0);
        done_size=i;
        done=done_storage.data();
    }
    
    //return if first point is the bordercolor
    if (qbg_active_page_offset[iy*qbg_width+ix]==bordercol) return;
    
    //create first node
    a_x[0]=ix; a_y[0]=iy;
    a_t[0]=15;
    //types:
    //&1=check left
    //&2=check right
    //&4=check above
    //&8=check below
    
    a_n=1;
    qbg_active_page_offset[iy*qbg_width+ix]=fillcol;
    done[iy*qbg_width+ix]=1;
    
    nextpass:
    b_n=0;
    for (i=0;i<a_n;i++){
        t=a_t[i]; ix=a_x[i]; iy=a_y[i];
        
        //left
        if (t&1){
            x2=ix-1; y2=iy;
            if (x2>=qbg_view_x1){
                offset=y2*qbg_width+x2;
                if (!done[offset]){
                    done[offset]=1;
                    if (qbg_active_page_offset[offset]!=bordercol){
                        qbg_active_page_offset[offset]=fillcol;
                        b_t[b_n]=13; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                    }}}}
                    
                    //right
                    if (t&2){
                        x2=ix+1; y2=iy;
                        if (x2<=qbg_view_x2){
                            offset=y2*qbg_width+x2;
                            if (!done[offset]){
                                done[offset]=1;
                                if (qbg_active_page_offset[offset]!=bordercol){
                                    qbg_active_page_offset[offset]=fillcol;
                                    b_t[b_n]=14; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                }}}}
                                
                                //above
                                if (t&4){
                                    x2=ix; y2=iy-1;
                                    if (y2>=qbg_view_y1){
                                        offset=y2*qbg_width+x2;
                                        if (!done[offset]){
                                            done[offset]=1;
                                            if (qbg_active_page_offset[offset]!=bordercol){
                                                qbg_active_page_offset[offset]=fillcol;
                                                b_t[b_n]=7; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                            }}}}
                                            
                                            //below
                                            if (t&8){
                                                x2=ix; y2=iy+1;
                                                if (y2<=qbg_view_y2){
                                                    offset=y2*qbg_width+x2;
                                                    if (!done[offset]){
                                                        done[offset]=1;
                                                        if (qbg_active_page_offset[offset]!=bordercol){
                                                            qbg_active_page_offset[offset]=fillcol;
                                                            b_t[b_n]=11; b_x[b_n]=x2; b_y[b_n]=y2; b_n++;//add new node
                                                        }}}}
                                                        
    }//i
    
    //no new nodes?
    if (b_n==0){
        memset(done,0,write_page->width*write_page->height);//cleanup
        return;//finished!
    }
    
    //swap a & b arrays
    sp=a_x; a_x=b_x; b_x=sp;
    sp=a_y; a_y=b_y; b_y=sp;
    cp=a_t; a_t=b_t; b_t=cp;
    a_n=b_n;
    
    goto nextpass;
    
}






//////////////////////////////////////////////////////////////////////////////////////////////////////////

void getptsize_1bpp (const qbs *pt, int32 *sx, int32 *sy) {
    *sx = 8;
    *sy = pt->len;
}

uint32 getptcol_1bpp (const qbs *pt, int32 x, int32 y) {
    return (pt->chr[y] >> (7-x)) & 1;
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////

void getptsize_2bpp (const qbs *pt, int32 *sx, int32 *sy) {
    *sx = 4;
    *sy = pt->len;
}

uint32 getptcol_2bpp (const qbs *pt, int32 x, int32 y) {
    return (pt->chr[y] >> ((3-x) << 1)) & 0x03;
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////

void getptsize_4bpp (const qbs *pt, int32 *sx, int32 *sy) {
    *sx = 8;
    *sy = pt->len >> 2;
    if (((*sy) << 2) < pt->len) ++*sy;
}

uint32 getptcol_4bpp (const qbs *pt, int32 x, int32 y) {
    int quadstart = (y << 2);
    uint32 clr = 0;
    for(uint8 i=0; i<4; ++i) {
        uint8 byte = ((i+quadstart) >= pt->len ? 0 : pt->chr[i+quadstart]);
        if (byte & (1 << (7-x)))
            clr |= (1 << i);
    }
    return clr;
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////

void getptsize_8bpp (const qbs *pt, int32 *sx, int32 *sy) {
    *sx = 1;
    *sy = pt->len;
}

uint32 getptcol_8bpp (const qbs *pt, int x, int y) {
    return pt->chr[y];
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////


//  MODIFIED BY:  The Joyful Programmer
//         DATE:  11/6/2022
//
//  NOTE:  THIS sub_paint FUNCTION WOULD NOT PAINT A PATTERN IN SCREEN 9 IF A bordercol WAS NOT
//         DEFINED IN CODE, LIKE "PAINT(100, 100), Tile$". SEE ADDITIONAL COMMENTS BELOW FOR
//         FURTHER INFORMATION.


void sub_paint(float x,float y,qbs *fillstr,uint32 bordercol,qbs *backgroundstr,int32 passed){
    if (new_error) return;
    
    //uses 2 buffers, a and b, and swaps between them for reading and creating
    static uint32 a_n{0};
    static std::vector<uint16> a_x_storage(65536),a_y_storage(65536);
    static std::vector<uint8> a_t_storage(65536);
    static uint32 b_n{0};
    static std::vector<uint16> b_x_storage(65536),b_y_storage(65536);
    static std::vector<uint8> b_t_storage(65536);
    static std::vector<uint8> done_storage(640*480,0);
    uint16 *a_x{a_x_storage.data()};
    uint16 *a_y{a_y_storage.data()};
    uint8 *a_t{a_t_storage.data()};
    uint16 *b_x{b_x_storage.data()};
    uint16 *b_y{b_y_storage.data()};
    uint8 *b_t{b_t_storage.data()};
    uint8 *done{done_storage.data()};
    static int32 ix;
    static int32 iy;
    static int32 i;
    static int32 t;
    static int32 x2;
    static int32 y2;
    static uint32 offset;
    static uint8 *cp;
    static uint16 *sp;
    static int32 done_size{640*480};

    if (qbg_text_only) {error(5); return;}
    if ((passed&2) == 0) {error(5); return;} //must be called with this parameter!
    if (fillstr->len == 0) {error(5); return;}

    static uint8 tile[8][64];
    static int32 sx;
    static int32 sy;
    
    void (*getptsize)(const qbs *pt, int32 *sx, int32 *sy) = getptsize_4bpp;
    uint32 (*getptcol)(const qbs *pt, int32 x, int32 y) = getptcol_4bpp;

    switch(write_page->compatible_mode) {
        case 1:
            getptsize = getptsize_2bpp;
            getptcol = getptcol_2bpp;
            break;
        case 2:
            getptsize = getptsize_1bpp;
            getptcol = getptcol_1bpp;
            break;
        case 7:
        case 8:
        case 9:
        case 12:
            getptsize = getptsize_4bpp;
            getptcol = getptcol_4bpp;
            break;
        case 13:
            getptsize = getptsize_8bpp;
            getptcol = getptcol_8bpp;
            break;
    }

    getptsize(fillstr, &sx, &sy);
    for(int x = 0; x < sx; ++x)
        for(int y = 0; y < sy; ++y)
            tile[x][y] = getptcol(fillstr, x, y);

    bordercol &= write_page->mask;
    
    if (passed & 1) {
        write_page->x += x;
        write_page->y += y;
    } else {
        write_page->x = x;
        write_page->y = y;
    }
    
    if (write_page->clipping_or_scaling) {
        if (write_page->clipping_or_scaling == 2) {
            ix = qbr_float_to_long(write_page->x *
                                   write_page->scaling_x +
                                   write_page->scaling_offset_x) +
                                   write_page->view_offset_x;
            iy = qbr_float_to_long(write_page->y *
                                   write_page->scaling_y + 
                                   write_page->scaling_offset_y) + 
                                   write_page->view_offset_y;
        } else {
            ix=qbr_float_to_long(write_page->x) + write_page->view_offset_x;
            iy=qbr_float_to_long(write_page->y) + write_page->view_offset_y;
        }
    }else{
        ix=qbr_float_to_long(write_page->x);
        iy=qbr_float_to_long(write_page->y);
    }
    
    //return if offscreen
    if ((ix < write_page->view_x1) || 
        (iy<write_page->view_y1) ||
        (ix>write_page->view_x2) || 
        (iy>write_page->view_y2))
            return;
    
    
    //overrides
    qbg_active_page_offset = write_page->offset;
    qbg_width = write_page->width;
    qbg_view_x1 = write_page->view_x1;
    qbg_view_y1 = write_page->view_y1;
    qbg_view_x2 = write_page->view_x2;
    qbg_view_y2 = write_page->view_y2;
    
    i = write_page->width * write_page->height;
    
    if (i > done_size){
        done_storage.assign(i, 0);
        done_size = i;
        done = done_storage.data();
    }

    //  FirstColor IS THE COLOR OF THE FIRST PIXEL CHECKED. IF bordercol WAS NOT PROVIDED,
    //  WHICH WOULD HAVE THE VALUE OF "NULL", THEN WE NEED TO CHECK AGAINST THE FIRST COLOR
    //  OBTAINED.
    uint32 FirstColor{qbg_active_page_offset[iy * qbg_width + ix] & write_page->mask};

    //  A NEW VARIABLE IS INTRODUCED HERE TO HELP REDUCE THE AMOUNT OF CODE LATER ON.
    bool SetColor{false};

    //  IF THE bordercol IS NULL, THEN THE CHECK FOR A MATCH WOULD ALWAYS RETURN FALSE.
    if (bordercol != NULL)
        //return if first point is the bordercolor.
        if (qbg_active_page_offset[iy * qbg_width + ix] == bordercol) return;
    
    //create first node
    a_x[0] = ix;
    a_y[0] = iy;
    a_t[0] = 15;
    //types:
    //&1=check left
    //&2=check right
    //&4=check above
    //&8=check below
    
    a_n = 1;
    qbg_active_page_offset[iy * qbg_width + ix] = tile[ix % sx][iy % sy];
    done[iy * qbg_width + ix] = 1;
    
    //  LABELS SHOULD BE AVOIDED IN C++
    //nextpass:

    //  WE CAN USE A WHILE LOOP TO CREATE AN INFINITE LOOP THAT CAN BE BROKEN OUT OF
    //  AS NEEDED.
    while (true) {

        b_n = 0;
        for (i = 0; i < a_n; i++) {

            //  CLEAR THE SetColor FLAG TO FALSE.
            SetColor = false;
            t = a_t[i];
            ix = a_x[i];
            iy = a_y[i];
            
            // left
            if (t & 1) {
                x2 = ix - 1;
                y2 = iy;
                if (x2 >= qbg_view_x1) {
                    offset = y2 * qbg_width + x2;
                    if (!done[offset]) {
                        done[offset] = 1;
                        if (bordercol != NULL) {
                            //  IF THE bordercol IS NOT NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  MATCHES THE bordercol VALUE. WE SET THE VALUE TO TRUE SO WE CAN
                            //  WE CAN RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] != bordercol) SetColor = true;
                        } else {
                            //  IF THE bordercol IS NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  IS THE SAME COLOR AS THE FirstColor VALUE. SET THE "SetColor"
                            //  FLAG TO TRUE TO RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] == FirstColor) SetColor = true;
                        }

                        //  BY USING THE SetColor VARIABLE, WE CAN PREVENT REPEATING OF CODE.
                        if (SetColor == true) {
                            b_t[b_n] = 13;
                            qbg_active_page_offset[offset] = tile[x2 % sx][y2 % sy];
                            b_x[b_n] = x2;
                            b_y[b_n] = y2;
                            b_n++; // add new node
                        }
                    }
                }
            }

            SetColor = false;

            // right
            if (t & 2) {
                x2 = ix + 1;
                y2 = iy;
                if (x2 <= qbg_view_x2) {
                    offset = y2 * qbg_width + x2;
                    if (!done[offset]) {
                        done[offset] = 1;
                        if (bordercol != NULL) {
                            //  IF THE bordercol IS NOT NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  MATCHES THE bordercol VALUE. WE SET THE VALUE TO TRUE SO WE CAN
                            //  WE CAN RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] != bordercol) SetColor = true;
                        } else {
                            //  IF THE bordercol IS NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  IS THE SAME COLOR AS THE FirstColor VALUE. SET THE "SetColor"
                            //  FLAG TO TRUE TO RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] == FirstColor) SetColor = true;
                        }

                        //  BY USING THE SetColor VARIABLE, WE CAN PREVENT REPEATING OF CODE.
                        if (SetColor == true) {
                            qbg_active_page_offset[offset] = tile[x2 % sx][y2 % sy];
                            b_t[b_n] = 14;
                            b_x[b_n] = x2;
                            b_y[b_n] = y2;
                            b_n++; // add new node
                        }
                    }
                }
            }

            SetColor = false;

            // above
            if (t & 4) {
                x2 = ix;
                y2 = iy - 1;
                if (y2 >= qbg_view_y1) {
                    offset = y2 * qbg_width + x2;
                    if (!done[offset]) {
                        done[offset] = 1;
                        if (bordercol != NULL) {
                            //  IF THE bordercol IS NOT NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  MATCHES THE bordercol VALUE. WE SET THE VALUE TO TRUE SO WE CAN
                            //  WE CAN RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] != bordercol) SetColor = true;
                        } else {
                            //  IF THE bordercol IS NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  IS THE SAME COLOR AS THE FirstColor VALUE. SET THE "SetColor"
                            //  FLAG TO TRUE TO RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] == FirstColor) SetColor = true;
                        }

                        //  BY USING THE SetColor VARIABLE, WE CAN PREVENT REPEATING OF CODE.
                        if (SetColor == true) {
                            qbg_active_page_offset[offset] = tile[x2 % sx][y2 % sy];
                            b_t[b_n] = 7;
                            b_x[b_n] = x2;
                            b_y[b_n] = y2;
                            b_n++; // add new node
                        }
                    }
                }
            }

            SetColor = false;

            // below
            if (t & 8) {
                x2 = ix;
                y2 = iy + 1;
                if (y2 <= qbg_view_y2) {
                    offset = y2 * qbg_width + x2;
                    if (!done[offset]) {
                        done[offset] = 1;
                        if (bordercol != NULL) {
                            //  IF THE bordercol IS NOT NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  MATCHES THE bordercol VALUE. WE SET THE VALUE TO TRUE SO WE CAN
                            //  WE CAN RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] != bordercol) SetColor = true;
                        } else {
                            //  IF THE bordercol IS NULL, WE CAN CHECK IF THE CURRENT PIXEL
                            //  IS THE SAME COLOR AS THE FirstColor VALUE. SET THE "SetColor"
                            //  FLAG TO TRUE TO RENDER THE PIXEL.
                            if (qbg_active_page_offset[offset] == FirstColor) SetColor = true;
                        }

                        //  BY USING THE SetColor VARIABLE, WE CAN PREVENT REPEATING OF CODE.
                        if (SetColor == true) {
                            qbg_active_page_offset[offset] = tile[x2 % sx][y2 % sy];
                            b_t[b_n] = 11;
                            b_x[b_n] = x2;
                            b_y[b_n] = y2;
                            b_n++; // add new node
                        }
                    }
                }
            }

        } // i

        // no new nodes?
        if (b_n == 0) {
            memset(done, 0, write_page->width * write_page->height); // cleanup
            return;                                                  // finished!
        }

        // swap a & b arrays
        sp = a_x;
        a_x = b_x;
        b_x = sp;
        sp = a_y;
        a_y = b_y;
        b_y = sp;
        cp = a_t;
        a_t = b_t;
        b_t = cp;
        a_n = b_n;

        //  THE GOTO STATEMENT SHOULD NEVER BE USED IN C++.
        //goto nextpass;
    }
}










void sub_circle(double x,double y,double r,uint32 col,double start,double end,double aspect,int32 passed){
    //                                                &2         &4           &8         &16
    //[{STEP}](?,?),?[,[?][,[?][,[?][,?]]]]
    if (new_error) return;
    
    
    
    //data
    static double pi= 3.1415926535897932,pi2=6.2831853071795865;
    static int32 line_to_start,line_from_end;
    static int32 ix,iy;//integer screen co-ordinates of circle's centre
    static double xspan,yspan;
    static double c;//circumference
    static double px,py;
    static double sinb,cosb;//second angle used in double-angle-formula
    static int32 pixels;
    static double tmp;
    static int32 tmpi;
    static int32 i;
    static int32 exclusive;
    static double arc1,arc2,arc3,arc4,arcinc;
    static double px2,py2;
    static int32 x2,y2;
    static int32 x3,y3;
    static int32 lastplotted_x2,lastplotted_y2;
    static int32 lastchecked_x2,lastchecked_y2;
    
    if (write_page->text){error(5); return;}
    
    //lines to & from centre
    if (!(passed&4)) start=0;
    if (!(passed&8)) end=pi2;
    line_to_start=0; if (start<0) {line_to_start=1; start=-start;}
    line_from_end=0; if (end<0) {line_from_end=1; end=-end;}
    
    //error checking
    if (start>pi2){error(5); return;}
    if (end>pi2){error(5); return;}
    
    //when end<start, the arc of the circle that wouldn't have been drawn if start & end
    //were swapped is drawn
    exclusive=0;
    if (end<start){
        tmp=start; start=end; end=tmp;
        tmpi=line_to_start; line_to_start=line_from_end; line_from_end=tmpi;
        exclusive=1;
    }
    
    //calc. centre
    if (passed&1){x=write_page->x+x; y=write_page->y+y;}
    write_page->x=x; write_page->y=y;//set graphics cursor position to circle's centre
    
    
    
    r=x+r;//the differece between x & x+r in pixels will be the radius in pixels
    //resolve coordinates (but keep as floats)
    if (write_page->clipping_or_scaling){
        if (write_page->clipping_or_scaling==2){
            x=x*write_page->scaling_x+write_page->scaling_offset_x+write_page->view_offset_x;
            y=y*write_page->scaling_y+write_page->scaling_offset_y+write_page->view_offset_y;
            r=r*write_page->scaling_x+write_page->scaling_offset_x+write_page->view_offset_x;
            }else{
            x=x+write_page->view_offset_x;
            y=y+write_page->view_offset_y;
            r=r+write_page->view_offset_x;
        }
    }
    if (x<0) ix=x-0.5; else ix=x+0.5;
    if (y<0) iy=y-0.5; else iy=y+0.5;
    r=fabs(r-x);//r is now a radius in pixels
    
    
    
    //adjust vertical and horizontal span of the circle based on aspect ratio
    xspan=r; yspan=r;
    if (!(passed&16)){
        aspect=1;//Note: default aspect ratio is 1:1 for QBNex specific modes (256/32)
        if (write_page->compatible_mode==1) aspect=4.0*(200.0/320.0)/3.0;
        if (write_page->compatible_mode==2) aspect=4.0*(200.0/640.0)/3.0;
        if (write_page->compatible_mode==7) aspect=4.0*(200.0/320.0)/3.0;
        if (write_page->compatible_mode==8) aspect=4.0*(200.0/640.0)/3.0;
        if (write_page->compatible_mode==9) aspect=4.0*(350.0/640.0)/3.0;
        if (write_page->compatible_mode==10) aspect=4.0*(350.0/640.0)/3.0;
        if (write_page->compatible_mode==11) aspect=4.0*(480.0/640.0)/3.0;
        if (write_page->compatible_mode==12) aspect=4.0*(480.0/640.0)/3.0;
        if (write_page->compatible_mode==13) aspect=4.0*(200.0/320.0)/3.0;
        //Old method: aspect=4.0*((double)write_page->height/(double)write_page->width)/3.0;
    }
    if (aspect>=0){
        if (aspect<1){
            //aspect: 0 to 1
            yspan*=aspect;
        }
        if (aspect>1){
            //aspect: 1 to infinity
            xspan/=aspect;
        }
        }else{
        if (aspect>-1){
            //aspect: -1 to 0
            yspan*=(1+aspect);
        }
        //if aspect<-1 no change is required
    }
    
    //skip everything if none of the circle is inside current viwport
    if ((x+xspan+0.5)<write_page->view_x1) return;
    if ((y+yspan+0.5)<write_page->view_y1) return;
    if ((x-xspan-0.5)>write_page->view_x2) return;
    if ((y-yspan-0.5)>write_page->view_y2) return;
    
    if (!(passed&2)) col=write_page->color;
    write_page->draw_color=col;
    
    //pre-set/pre-calculate values
    c=pi2*r;
    pixels=c/4.0+0.5;
    arc1=0;
    arc2=pi;
    arc3=pi;
    arc4=pi2;
    arcinc=(pi/2)/(double)pixels;
    sinb=sin(arcinc);
    cosb=cos(arcinc);
    lastplotted_x2=-1;
    lastchecked_x2=-1;
    i=0;
    
    if (line_to_start){
        px=cos(start); py=sin(start);
        x2=px*xspan+0.5; y2=py*yspan-0.5;
        fast_line(ix,iy,ix+x2,iy-y2,col);
    }
    
    px=1;
    py=0;
    
    drawcircle:
    x2=px*xspan+0.5;
    y2=py*yspan-0.5;
    
    if (i==0) {lastchecked_x2=x2; lastchecked_y2=y2; goto plot;}
    
    if ( (abs(x2-lastplotted_x2)>=2)||(abs(y2-lastplotted_y2)>=2) ){
        plot:
        if (exclusive){
            if ((arc1<=start)||(arc1>=end)){pset_and_clip(ix+lastchecked_x2,iy+lastchecked_y2,col);}
            if ((arc2<=start)||(arc2>=end)){pset_and_clip(ix-lastchecked_x2,iy+lastchecked_y2,col);}
            if ((arc3<=start)||(arc3>=end)){pset_and_clip(ix-lastchecked_x2,iy-lastchecked_y2,col);}
            if ((arc4<=start)||(arc4>=end)){pset_and_clip(ix+lastchecked_x2,iy-lastchecked_y2,col);}
            }else{//inclusive
            if ((arc1>=start)&&(arc1<=end)){pset_and_clip(ix+lastchecked_x2,iy+lastchecked_y2,col);}
            if ((arc2>=start)&&(arc2<=end)){pset_and_clip(ix-lastchecked_x2,iy+lastchecked_y2,col);}
            if ((arc3>=start)&&(arc3<=end)){pset_and_clip(ix-lastchecked_x2,iy-lastchecked_y2,col);}
            if ((arc4>=start)&&(arc4<=end)){pset_and_clip(ix+lastchecked_x2,iy-lastchecked_y2,col);}
        }
        if (i>pixels) goto allplotted;
        lastplotted_x2=lastchecked_x2; lastplotted_y2=lastchecked_y2;
    }
    lastchecked_x2=x2; lastchecked_y2=y2;
    
    if (i<=pixels){
        i++;
        if (i>pixels) goto plot;
        px2=px*cosb+py*sinb;
        py=py*cosb-px*sinb;
        px=px2;
        if (i) {arc1+=arcinc; arc2-=arcinc; arc3+=arcinc; arc4-=arcinc;}
        goto drawcircle;
    }
    allplotted:
    
    if (line_from_end){
        px=cos(end); py=sin(end);
        x2=px*xspan+0.5; y2=py*yspan-0.5;
        fast_line(ix,iy,ix+x2,iy-y2,col);
    }
    
}//sub_circle



uint32 point(int32 x,int32 y){//does not clip!
    if (read_page->bytes_per_pixel==1){
        return read_page->offset[y*read_page->width+x]&read_page->mask;
        }else{
        return read_page->offset32[y*read_page->width+x];
    }
    return NULL;
}




double func_point(float x,float y,int32 passed){
    static int32 x2,y2,i;
    
    if (!passed){
        if (write_page->text){error(5); return 0;}
        i=qbr_float_to_long(x);
        if ((i<0)||(i>3)){error(5); return 0;}
        switch(i){
            case 0:
            if (write_page->clipping_or_scaling==2){
                return qbr_float_to_long(write_page->x*write_page->scaling_x+write_page->scaling_offset_x);
            }
            return qbr_float_to_long(write_page->x);
            break;
            case 1:
            if (write_page->clipping_or_scaling==2){
                return qbr_float_to_long(write_page->y*write_page->scaling_y+write_page->scaling_offset_y);
            }
            return qbr_float_to_long(write_page->y);
            break;
            case 2:
            return write_page->x;
            break;
            case 3:
            return write_page->y;
            break;
            default:
            error(5);
            return 0;
        }
    }//!passed
    
    if (read_page->text){error(5); return 0;}
    if (read_page->clipping_or_scaling){
        if (read_page->clipping_or_scaling==2){
            x2=qbr_float_to_long(x*read_page->scaling_x+read_page->scaling_offset_x)+read_page->view_offset_x;
            y2=qbr_float_to_long(y*read_page->scaling_y+read_page->scaling_offset_y)+read_page->view_offset_y;
            }else{
            x2=qbr_float_to_long(x)+read_page->view_offset_x; y2=qbr_float_to_long(y)+read_page->view_offset_y;
        }
        }else{
        x2=qbr_float_to_long(x); y2=qbr_float_to_long(y);
    }
    if (x2>=read_page->view_x1){ 
        if (x2<=read_page->view_x2){
            if (y2>=read_page->view_y1){ 
                if (y2<=read_page->view_y2){
                    return point(x2,y2);
                }
            }
        }
    }
    return -1;
}








void sub_pset(float x,float y,uint32 col,int32 passed){
    if (new_error) return;
    static int32 x2,y2;
    if (!write_page->compatible_mode){error(5); return;}
    //Special Format: [{STEP}](?,?),[?]
    if (passed&1){write_page->x+=x; write_page->y+=y;}else{write_page->x=x; write_page->y=y;}
    if (!(passed&2)) col=write_page->color;
    write_page->draw_color=col;
    if (write_page->clipping_or_scaling){
        if (write_page->clipping_or_scaling==2){
            x2=qbr(write_page->x*write_page->scaling_x+write_page->scaling_offset_x)+write_page->view_offset_x;
            y2=qbr(write_page->y*write_page->scaling_y+write_page->scaling_offset_y)+write_page->view_offset_y;
            }else{
            x2=qbr(write_page->x)+write_page->view_offset_x; y2=qbr(write_page->y)+write_page->view_offset_y;
        }
        if (x2>=write_page->view_x1){ if (x2<=write_page->view_x2){
            if (y2>=write_page->view_y1){ if (y2<=write_page->view_y2){
                pset(x2,y2,col);
            }}}}
            return;
            }else{
            x2=qbr(write_page->x); if (x2>=0){ if (x2<write_page->width){
                y2=qbr(write_page->y); if (y2>=0){ if (y2<write_page->height){
                    pset(x2,y2,col);
                }}}}
    }
    return;
}

void sub_preset(float x,float y,uint32 col,int32 passed){
    if (new_error) return;
    if (!(passed&2)){
        col=write_page->background_color;
        passed|=2;
    }
    sub_pset(x,y,col,passed);
    return;
}
