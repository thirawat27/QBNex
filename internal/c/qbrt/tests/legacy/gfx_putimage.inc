// Verbatim copy of legacy internal/c/libqb.cpp (oracle commit 545b859), lines 4240-5390.
// Test-only reference oracle for tests/diff_gfx.cpp; production code is internal/c/qbrt/src/gfx/putimage.rs.
void sub__putimage(double f_dx1,double f_dy1,double f_dx2,double f_dy2,int32 src,int32 dst,double f_sx1,double f_sy1,double f_sx2,double f_sy2,int32 passed){
    
    /* 
        Format & passed bits: (needs updating)
        [[{STEP}](?,?)[-[{STEP}](?,?)]][,[?][,[?][,[[{STEP}](?,?)[-[{STEP}](?,?)]][,{_SMOOTH}]]]]
        2?     1              4?       8                                 512      128
    */
    
    static int32 w,h,sskip,dskip,x,y,xx,yy,z,x2,y2,dbpp,sbpp;
    static img_struct *s,*d;
    static uint32 *soff32,*doff32,col,clearcol,destcol;
    static uint8 *soff,*doff;
    static uint8 *cp;
    static int32 xdir,ydir,no_stretch,no_clip,no_reverse,flip,mirror;
    static double mx,my,fx,fy,fsx1,fsy1,fsx2,fsy2,dv,dv2;
    static int32 sx1,sy1,sx2,sy2,dx1,dy1,dx2,dy2;
    static int32 sw,sh,dw,dh;
    static uint32 *pal;
    static uint32 *ulp;
    
    no_stretch=0; no_clip=0; no_reverse=1;
    flip=0; mirror=0;
    
    static int32 use_hardware;
    static img_struct s_emu,d_emu;//used to emulate a source/dest image for calculation purposes
    use_hardware=0;
    
    //is source a hardware handle?
    hardware_img_struct* dst_himg=NULL;
    hardware_img_struct* src_himg=NULL;
    if (src){
        src_himg=(hardware_img_struct*)list_get(hardware_img_handles,src-HARDWARE_IMG_HANDLE_OFFSET);
        if (src_himg!=NULL){//source is hardware image
            src-=HARDWARE_IMG_HANDLE_OFFSET;
            
            flush_old_hardware_commands();
            
            s_emu.width=src_himg->w;
            s_emu.height=src_himg->h;
            s_emu.clipping_or_scaling=0;
            s_emu.alpha_disabled=src_himg->alpha_disabled;
            s=&s_emu;
            
            //check dst      
            if (dst<0){
                dst_himg=(hardware_img_struct*)list_get(hardware_img_handles,dst-HARDWARE_IMG_HANDLE_OFFSET);
                if (dst_himg==NULL){error(258); return;}
                dst-=HARDWARE_IMG_HANDLE_OFFSET;
                
                d_emu.width=dst_himg->w;
                d_emu.height=dst_himg->h;
                d_emu.clipping_or_scaling=0;
                d_emu.alpha_disabled=dst_himg->alpha_disabled;
                d=&d_emu;
                
                }else{
                if (dst>1) {error(5); return;}
                dst=-dst;
                
                d=display_page;//use parameters from display page
                
            }
            
            sbpp=4; dbpp=4;
            use_hardware=1;
            
            goto resolve_coordinates;
            
        }//source is hardware image
    }//src passed
    
    
    
    if (passed&8){//src
        //validate
        if (src>=0){
            validatepage(src); s=&img[page[src]];
            }else{
            src=-src;
            if (src>=nextimg){error(258); return;}
            s=&img[src];
            if (!s->valid){error(258); return;}
        }
        }else{
        s=read_page;
    }//src
    if (s->text){error(5); return;}
    sbpp=s->bytes_per_pixel;
    
    if (passed&32){//dst
        //validate
        if (dst>=0){
            validatepage(dst); d=&img[page[dst]];
            }else{
            dst=-dst;
            if (dst>=nextimg){error(258); return;}
            d=&img[dst];
            if (!d->valid){error(258); return;}
        }
        }else{
        d=write_page;
    }//dst
    if (d->text){error(5); return;}
    dbpp=d->bytes_per_pixel;
    if ((sbpp==4)&&(dbpp==1)){error(5); return;}
    if (s==d){
        //{error(5); return;}//cannot put source onto itself!
        int32 temphandle=func__copyimage(dst,NULL,0);
        passed=passed|8; //make certain we set the flag TO LET QBNex know we're passing a handle to the temp image
        sub__putimage(f_dx1, f_dy1, f_dx2, f_dy2,temphandle, dst, f_sx1, f_sy1, f_sx2, f_sy2, passed);
        sub__freeimage(temphandle,1);
        return;
    }
    
    
    resolve_coordinates:
    
    
    
    //quick references
    sw=s->width; sh=s->height; dw=d->width; dh=d->height;
    
    
    
    //change coordinates according to step
    if (passed&2){f_dx1=d->x+f_dx1; f_dy1=d->y+f_dy1;}
    if (passed&16){f_dx2=f_dx1+f_dx2; f_dy2=f_dy1+f_dy2;}
    if (passed&256){f_sx1=s->x+f_sx1; f_sy1=s->y+f_sy1;}
    if (passed&1024){f_sx2=f_sx1+f_sx2; f_sy2=f_sy1+f_sy2;}
    
    //Here we calculate what our final point is going to be and put that value into the _DEST x/y so we can get STEP back correctly on the next call.
    //or something like that...  I have no idea how to explain what the heck I'm gdoing here!
    //Basically I'm just trying to update the x/y point that we last plot to on our screen so we can pick it back up and use it again...
    if (passed&4){
        //we entered both dest numbers.  Our last point plotted should be f_dx2/f_dy2
        d->x=f_dx2; 
        d->y=f_dy2;
    }
    else{
        if (passed&1){
            //we only sent it the first dest value.  We want to put our rectangle on a portion of the screen starting at this point
            if (passed&512) {
                //we have all the source values.  We want to put that rectangle over to dest starting at that point
                d->x=f_dx1+abs(f_sx2-f_sx1); 
                d->y=f_dy1+abs(f_sy2-f_sy1);
            }
            else{
                //we want to go from f_sx1,F_sx2 to the edge of the screen and put it over to dest starting at that point
                d->x=f_dx1+abs(sw-f_sx1); 
                d->y=f_dy1+abs(sh-f_sy1);
            }
        }
        else{
            //we never sent the first source value.  We want to put the image over the whole screen.
            d->x=dw; 
            d->y=dh;
        }
    }
    
    //And here we update our source page information so the STEP will work properly there as well.
    //This seems a little simpler logic
    
    if (passed&512){
        //we sent it the stop coordinate of where we're reading from
        s->x = f_sx2;
        s->y = f_sy2;
    }
    else{
        //we didn't and we need to have it copy from wherever the starting point is to the bottom right of the screen.
        //so our final point read will be the source width/height
        s->x = sw;
        s->y = sh;
    }
    
    
    //resolve coordinates
    if (passed&1){//dx1,dy1
        if (d->clipping_or_scaling){
            if (d->clipping_or_scaling==2){
                dx1=qbr_float_to_long(f_dx1*d->scaling_x+d->scaling_offset_x)+d->view_offset_x;
                dy1=qbr_float_to_long(f_dy1*d->scaling_y+d->scaling_offset_y)+d->view_offset_y;
                }else{
                dx1=qbr_float_to_long(f_dx1)+d->view_offset_x; dy1=qbr_float_to_long(f_dy1)+d->view_offset_y;
            }
            }else{
            dx1=qbr_float_to_long(f_dx1); dy1=qbr_float_to_long(f_dy1);
        }
        //note: dx2 & dy2 cannot be passed if dx1 & dy1 weren't passed
        if (passed&4){//dx2,dy2
            if (d->clipping_or_scaling){
                if (d->clipping_or_scaling==2){
                    dx2=qbr_float_to_long(f_dx2*d->scaling_x+d->scaling_offset_x)+d->view_offset_x;
                    dy2=qbr_float_to_long(f_dy2*d->scaling_y+d->scaling_offset_y)+d->view_offset_y;
                    }else{
                    dx2=qbr_float_to_long(f_dx2)+d->view_offset_x; dy2=qbr_float_to_long(f_dy2)+d->view_offset_y;
                }
                }else{
                dx2=qbr_float_to_long(f_dx2); dy2=qbr_float_to_long(f_dy2);
            }
            }else{//dx2,dy2
            dx2=0; dy2=0;
        }//dx2,dy2
        }else{//dx1,dy1
        dx1=0; dy1=0; dx2=0; dy2=0;
    }//dx1,dy1
    
    if (passed&64){//sx1,sy1
        if (s->clipping_or_scaling){
            
            if (s->clipping_or_scaling==2){
                sx1=qbr_float_to_long(f_sx1*s->scaling_x+s->scaling_offset_x)+s->view_offset_x;
                sy1=qbr_float_to_long(f_sy1*s->scaling_y+s->scaling_offset_y)+s->view_offset_y;
                }else{
                sx1=qbr_float_to_long(f_sx1)+s->view_offset_x; sy1=qbr_float_to_long(f_sy1)+s->view_offset_y;
            }
            }else{
            sx1=qbr_float_to_long(f_sx1); sy1=qbr_float_to_long(f_sy1);
        }
        //note: sx2 & sy2 cannot be passed if sx1 & sy1 weren't passed
        if (passed&512){//sx2,sy2
            if (s->clipping_or_scaling){
                if (s->clipping_or_scaling==2){
                    sx2=qbr_float_to_long(f_sx2*s->scaling_x+s->scaling_offset_x)+s->view_offset_x;
                    sy2=qbr_float_to_long(f_sy2*s->scaling_y+s->scaling_offset_y)+s->view_offset_y;
                    }else{
                    sx2=qbr_float_to_long(f_sx2)+s->view_offset_x; sy2=qbr_float_to_long(f_sy2)+s->view_offset_y;
                }
                }else{
                sx2=qbr_float_to_long(f_sx2); sy2=qbr_float_to_long(f_sy2);
            }
            }else{//sx2,sy2
            sx2=0; sy2=0;
        }//sx2,sy2
        }else{//sx1,sy1
        sx1=0; sy1=0; sx2=0; sy2=0;
    }//sx1,sy1
    
    //all co-ordinates resolved (but omitted co-ordinates are set to 0!)
    
    if (use_hardware){
        //calculate omitted co-ordinates
        if ((passed&4)&&(passed&512)) goto got_hw_coord;//all passed
        if (passed&4){//(dx1,dy1)-(dx2,dy2)...
            if (passed&64){//(dx1,dy1)-(dx2,dy2),...,(sx1,sy1)
                sx2=sx1+abs(dx2-dx1); sy2=sy1+abs(dy2-dy1);
                goto got_hw_coord;
                }else{//(dx1,dy1)-(dx2,dy2)
                sx2=sw-1; sy2=sh-1;
                goto got_hw_coord;
            }
        }
        if (passed&512){//...(sx1,sy1)-(sx2,sy2)
            if (passed&1){//(dx1,dy1),,(sx1,sy1)-(sx2,sy2)
                dx2=dx1+abs(sx2-sx1); dy2=dy1+abs(sy2-sy1);
                goto got_hw_coord;
                }else{//(sx1,sy1)-(sx2,sy2)
                dx2=dw-1; dy2=dh-1;
                goto got_hw_coord;
            }
        }
        if (passed&64){error(5); return;}//Invalid: NULL-NULL,?,?,(sx1,sy1)-NULL
        if (passed&1){//(dx1,dy1)
            sx2=s->width-1; sy2=s->height-1;
            dx2=dx1+sx2; dy2=dy1+sy2;
            goto got_hw_coord;
        }
        //no coords provided
        sx2=s->width-1; sy2=s->height-1;
        dx2=d->width-1; dy2=d->height-1;
        got_hw_coord:
        
        //create new command handle & structure
        int32 hgch=list_add(hardware_graphics_command_handles);
        hardware_graphics_command_struct* hgc=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,hgch);
        
        hgc->remove=0;
        
        //set command values
        hgc->command=HARDWARE_GRAPHICS_COMMAND__PUTIMAGE;
        
        hgc->src_img=src;
        hgc->src_x1=sx1;
        hgc->src_y1=sy1;
        hgc->src_x2=sx2;
        hgc->src_y2=sy2;
        
        hgc->dst_img=dst;
        hgc->dst_x1=dx1;
        hgc->dst_y1=dy1;
        hgc->dst_x2=dx2;
        hgc->dst_y2=dy2;
        
        hgc->smooth=0;//unless specified, no filtering will be applied
        if (passed&128) hgc->smooth=1;
        
        hgc->use_alpha=1;
        if (s->alpha_disabled) hgc->use_alpha=0;
        //only consider dest alpha setting if it is a hardware image    
        if (dst_himg){
            if (d->alpha_disabled) hgc->use_alpha=0;
        }
        
        //queue the command
        hgc->next_command=0;
        hgc->order=display_frame_order_next;
        
        if (last_hardware_command_added){
            hardware_graphics_command_struct* hgc2=(hardware_graphics_command_struct*)list_get(hardware_graphics_command_handles,last_hardware_command_added);
            hgc2->next_command=hgch;
        }
        last_hardware_command_added=hgch;
        if (first_hardware_command==0) first_hardware_command=hgch;
        
        return;
        
    }//use hardware
    
    //(decided not to throw error, QBNex will use linear filtering if/when available)
    //if (passed&128){error(5); return;}//software surfaces do not support pixel _SMOOTHing yet
    
    if ((passed&4)&&(passed&512)){//all co-ords given
        //could be stretched
        if ( (abs(dx2-dx1)==abs(sx2-sx1)) && (abs(dy2-dy1)==abs(sy2-sy1)) ){//non-stretched
            //could be flipped/reversed
            //could need clipping
            goto reverse;
        }
        goto stretch;
    }
    
    if (passed&4){//(dx1,dy1)-(dx2,dy2)...
        if (passed&64){//(dx1,dy1)-(dx2,dy2),...,(sx1,sy1)
            sx2=sx1+abs(dx2-dx1); sy2=sy1+abs(dy2-dy1);
            //can't be stretched
            //could be flipped/reversed
            //could need clipping
            goto reverse;
            }else{//(dx1,dy1)-(dx2,dy2)
            sx2=sw-1; sy2=sh-1;
            //could be stretched
            if ( ((abs(dx2-dx1)+1)==sw) && ((abs(dy2-dy1)+1)==sh) ){//non-stretched
                //could be flipped/reversed
                //could need clipping
                goto reverse;
            }
            goto stretch;
        }//16
    }//2
    
    if (passed&512){//...(sx1,sy1)-(sx2,sy2)
        if (passed&1){//(dx1,dy1),,(sx1,sy1)-(sx2,sy2)
            dx2=dx1+abs(sx2-sx1); dy2=dy1+abs(sy2-sy1);
            //can't be stretched
            //could be flipped/reversed
            //could need clipping
            goto reverse;
            }else{//(sx1,sy1)-(sx2,sy2)
            dx2=dw-1; dy2=dh-1;
            //could be stretched
            if ( ((abs(sx2-sx1)+1)==dw) && ((abs(sy2-sy1)+1)==dh) ){//non-stretched
                //could be flipped/reversed
                //could need clipping
                goto reverse;
            }
            goto stretch;
        }//1
    }//32
    
    if (passed&64){error(5); return;}//Invalid: NULL-NULL,?,?,(sx1,sy1)-NULL
    
    if (passed&1){//(dx1,dy1)
        sx2=s->width-1; sy2=s->height-1;
        dx2=dx1+sx2; dy2=dy1+sy2;
        goto clip;
    }
    
    //no co-ords given
    sx2=s->width-1; sy2=s->height-1;
    dx2=d->width-1; dy2=d->height-1;
    if ((sx2==dx2)&&(sy2==dy2)){//non-stretched
        //note: because 0-size image is illegal, no null size check is necessary
        goto noflip;//cannot be reversed
    }
    //precalculate required values
    w=dx2-dx1; h=dy2-dy1;
    fsx1=sx1; fsy1=sy1; fsx2=sx2; fsy2=sy2;
    //"pull" corners so all source pixels are evenly represented in dest rect
    if (fsx1<=fsx2){fsx1-=0.499999; fsx2+=0.499999;}else{fsx1+=0.499999; fsx2-=0.499999;}
    if (fsy1<=fsy2){fsy1-=0.499999; fsy2+=0.499999;}else{fsy1+=0.499999; fsy2-=0.499999;}
    //calc source gradients
    if (w) mx=(fsx2-fsx1)/((double)w); else mx=0.0;
    if (h) my=(fsy2-fsy1)/((double)h); else my=0.0;
    //note: mx & my represent the amount of change per dest pixel
    goto stretch_noreverse_noclip;
    
    stretch:
    //stretch is required
    
    //mirror?
    if (dx2<dx1){
        if (sx2>sx1) mirror=1;
    }
    if (sx2<sx1){
        if (dx2>dx1) mirror=1;
    }
    if (dx2<dx1){x=dx1; dx1=dx2; dx2=x;}
    if (sx2<sx1){x=sx1; sx1=sx2; sx2=x;}
    //flip?
    if (dy2<dy1){
        if (sy2>sy1) flip=1;
    }
    if (sy2<sy1){
        if (dy2>dy1) flip=1;
    }
    if (dy2<dy1){y=dy1; dy1=dy2; dy2=y;}
    if (sy2<sy1){y=sy1; sy1=sy2; sy2=y;}
    
    w=dx2-dx1; h=dy2-dy1;
    fsx1=sx1; fsy1=sy1; fsx2=sx2; fsy2=sy2;
    //"pull" corners so all source pixels are evenly represented in dest rect
    if (fsx1<=fsx2){fsx1-=0.499999; fsx2+=0.499999;}else{fsx1+=0.499999; fsx2-=0.499999;}
    if (fsy1<=fsy2){fsy1-=0.499999; fsy2+=0.499999;}else{fsy1+=0.499999; fsy2-=0.499999;}
    //calc source gradients
    if (w) mx=(fsx2-fsx1)/((double)w); else mx=0.0;
    if (h) my=(fsy2-fsy1)/((double)h); else my=0.0;
    //note: mx & my represent the amount of change per dest pixel
    
    //crop dest offscreen pixels
    if (dx1<0){
        if (mirror) fsx2+=((double)dx1)*mx; else fsx1-=((double)dx1)*mx;
        dx1=0;
    }
    if (dy1<0){
        if (flip) fsy2+=((double)dy1)*my; else fsy1-=((double)dy1)*my;
        dy1=0;
    }
    if (dx2>=dw){
        if (mirror) fsx1+=((double)(dx2-dw+1))*mx; else fsx2-=((double)(dx2-dw+1))*mx;
        dx2=dw-1;
    }
    if (dy2>=dh){
        if (flip) fsy1+=((double)(dy2-dh+1))*my; else fsy2-=((double)(dy2-dh+1))*my;
        dy2=dh-1;
    }
    //crop source offscreen pixels
    if (w){//gradient cannot be 0
        if (fsx1<-0.4999999){
            x=(-fsx1-0.499999)/mx+1.0;
            if (mirror) dx2-=x; else dx1+=x;
            fsx1+=((double)x)*mx;
        }
        if (fsx2>(((double)sw)-0.5000001)){
            x=(fsx2-(((double)sw)-0.500001))/mx+1.0;
            if (mirror) dx1+=x; else dx2-=x;
            fsx2-=(((double)x)*mx);
        }
    }//w
    if (h){//gradient cannot be 0
        if (fsy1<-0.4999999){
            y=(-fsy1-0.499999)/my+1.0;
            if (flip) dy2-=y; else dy1+=y;
            fsy1+=((double)y)*my;
        }
        if (fsy2>(((double)sh)-0.5000001)){
            y=(fsy2-(((double)sh)-0.500001))/my+1.0;
            if (flip) dy1+=y; else dy2-=y;
            fsy2-=(((double)y)*my);
        }
    }//h
    //<0-size/offscreen?
    //note: <0-size will cause reversal of dest
    //      offscreen values will result in reversal of dest
    if (dx1>dx2) return;
    if (dy1>dy2) return;
    //all values are now within the boundries of the source & dest
    
    stretch_noreverse_noclip:
    w=dx2-dx1+1; h=dy2-dy1+1;//recalculate based on actual number of pixels
    
    if (sbpp==4){
        if (s->alpha_disabled||d->alpha_disabled) goto put_32_noalpha_stretch;
        goto put_32_stretch;
    }
    if (dbpp==1){
        if (s->transparent_color==-1) goto put_8_stretch;
        goto put_8_clear_stretch;
    }
    if (s->transparent_color==-1) goto put_8_32_stretch;
    goto put_8_32_clear_stretch;
    
    put_32_stretch:
    //calc. starting points & change values
    if (flip){
        if (mirror){
            doff32=d->offset32+(dy2*dw+dx2);
            dskip=-dw+w;
            }else{
            doff32=d->offset32+(dy2*dw+dx1);
            dskip=-dw-w;
        }
        }else{
        if (mirror){
            doff32=d->offset32+(dy1*dw+dx2);
            dskip=dw+w;
            }else{
            doff32=d->offset32+(dy1*dw+dx1);
            dskip=dw-w;
        }
    }
    if (mirror) xdir=-1; else xdir=1;
    //plot rect
    yy=h;
    fy=fsy1;
    fsx1-=mx;//prev value is moved on from
    do{
        xx=w;
        ulp=s->offset32+sw*qbr_double_to_long(fy);
        fx=fsx1;
        do{
            //--------plot pixel--------
            switch((col=*(ulp+qbr_double_to_long(fx+=mx)))&0xFF000000){
                case 0xFF000000:
                *doff32=col;
                break;
                case 0x0:
                break;
                case 0x80000000:
                *doff32=(((*doff32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
                break; 
                case 0x7F000000:
                *doff32=(((*doff32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
                break;
                default:
                destcol=*doff32;
                cp=cblend+(col>>24<<16);
                *doff32=
                cp[(col<<8&0xFF00)+(destcol&255)    ]
                +(cp[(col&0xFF00)   +(destcol>>8&255) ]<<8)
                +(cp[(col>>8&0xFF00)+(destcol>>16&255)]<<16)
                +(ablend[(col>>24)+(destcol>>16&0xFF00)]<<24);
            };//switch
            //--------done plot pixel--------
            doff32+=xdir;
        }while(--xx);
        doff32+=dskip;
        fy+=my;
    }while(--yy);
    return;
    
    put_32_noalpha_stretch:
    //calc. starting points & change values
    if (flip){
        if (mirror){
            doff32=d->offset32+(dy2*dw+dx2);
            dskip=-dw+w;
            }else{
            doff32=d->offset32+(dy2*dw+dx1);
            dskip=-dw-w;
        }
        }else{
        if (mirror){
            doff32=d->offset32+(dy1*dw+dx2);
            dskip=dw+w;
            }else{
            doff32=d->offset32+(dy1*dw+dx1);
            dskip=dw-w;
        }
    }
    if (mirror) xdir=-1; else xdir=1;
    //plot rect
    yy=h;
    fy=fsy1;
    fsx1-=mx;//prev value is moved on from
    doff32-=xdir;
    do{
        xx=w;
        ulp=s->offset32+sw*qbr_double_to_long(fy);
        fx=fsx1;
        do{
            *(doff32+=xdir)=*(ulp+qbr_double_to_long(fx+=mx));
        }while(--xx);
        doff32+=dskip;
        fy+=my;
    }while(--yy);
    return;
    
    put_8_stretch:
    //calc. starting points & change values
    if (flip){
        if (mirror){
            doff=d->offset+(dy2*dw+dx2);
            dskip=-dw+w;
            }else{
            doff=d->offset+(dy2*dw+dx1);
            dskip=-dw-w;
        }
        }else{
        if (mirror){
            doff=d->offset+(dy1*dw+dx2);
            dskip=dw+w;
            }else{
            doff=d->offset+(dy1*dw+dx1);
            dskip=dw-w;
        }
    }
    if (mirror) xdir=-1; else xdir=1;
    //plot rect
    yy=h;
    fy=fsy1;
    fsx1-=mx;//prev value is moved on from
    doff-=xdir;
    do{
        xx=w;
        cp=s->offset+sw*qbr_double_to_long(fy);
        fx=fsx1;
        do{
            *(doff+=xdir)=*(cp+qbr_double_to_long(fx+=mx));
        }while(--xx);
        doff+=dskip;
        fy+=my;
    }while(--yy);
    return;
    
    put_8_clear_stretch:
    clearcol=s->transparent_color;
    //calc. starting points & change values
    if (flip){
        if (mirror){
            doff=d->offset+(dy2*dw+dx2);
            dskip=-dw+w;
            }else{
            doff=d->offset+(dy2*dw+dx1);
            dskip=-dw-w;
        }
        }else{
        if (mirror){
            doff=d->offset+(dy1*dw+dx2);
            dskip=dw+w;
            }else{
            doff=d->offset+(dy1*dw+dx1);
            dskip=dw-w;
        }
    }
    if (mirror) xdir=-1; else xdir=1;
    //plot rect
    yy=h;
    fy=fsy1;
    fsx1-=mx;//prev value is moved on from
    do{
        xx=w;
        cp=s->offset+sw*qbr_double_to_long(fy);
        fx=fsx1;
        do{
            if ((col=*(cp+qbr_double_to_long(fx+=mx)))!=clearcol){
                *doff=col;
            }
            doff+=xdir;
        }while(--xx);
        doff+=dskip;
        fy+=my;
    }while(--yy);
    return;
    
    put_8_32_stretch:
    pal=s->pal;
    //calc. starting points & change values
    if (flip){
        if (mirror){
            doff32=d->offset32+(dy2*dw+dx2);
            dskip=-dw+w;
            }else{
            doff32=d->offset32+(dy2*dw+dx1);
            dskip=-dw-w;
        }
        }else{
        if (mirror){
            doff32=d->offset32+(dy1*dw+dx2);
            dskip=dw+w;
            }else{
            doff32=d->offset32+(dy1*dw+dx1);
            dskip=dw-w;
        }
    }
    if (mirror) xdir=-1; else xdir=1;
    //plot rect
    yy=h;
    fy=fsy1;
    fsx1-=mx;//prev value is moved on from
    doff32-=xdir;
    do{
        xx=w;
        cp=s->offset+sw*qbr_double_to_long(fy);
        fx=fsx1;
        do{
            *(doff32+=xdir)=pal[*(cp+qbr_double_to_long(fx+=mx))];
        }while(--xx);
        doff32+=dskip;
        fy+=my;
    }while(--yy);
    return;
    
    put_8_32_clear_stretch:
    clearcol=s->transparent_color;
    pal=s->pal;
    //calc. starting points & change values
    if (flip){
        if (mirror){
            doff32=d->offset32+(dy2*dw+dx2);
            dskip=-dw+w;
            }else{
            doff32=d->offset32+(dy2*dw+dx1);
            dskip=-dw-w;
        }
        }else{
        if (mirror){
            doff32=d->offset32+(dy1*dw+dx2);
            dskip=dw+w;
            }else{
            doff32=d->offset32+(dy1*dw+dx1);
            dskip=dw-w;
        }
    }
    if (mirror) xdir=-1; else xdir=1;
    //plot rect
    yy=h;
    fy=fsy1;
    fsx1-=mx;//prev value is moved on from
    do{
        xx=w;
        cp=s->offset+sw*qbr_double_to_long(fy);
        fx=fsx1;
        do{
            if ((col=*(cp+qbr_double_to_long(fx+=mx)))!=clearcol){
                *doff32=pal[col];
            }
            doff32+=xdir;
        }while(--xx);
        doff32+=dskip;
        fy+=my;
    }while(--yy);
    return;
    
    reverse:
    //mirror?
    if (dx2<dx1){
        if (sx2>sx1) mirror=1;
    }
    if (sx2<sx1){
        if (dx2>dx1) mirror=1;
    }
    if (dx2<dx1){x=dx1; dx1=dx2; dx2=x;}
    if (sx2<sx1){x=sx1; sx1=sx2; sx2=x;}
    //flip?
    if (dy2<dy1){
        if (sy2>sy1) flip=1;
    }
    if (sy2<sy1){
        if (dy2>dy1) flip=1;
    }
    if (dy2<dy1){y=dy1; dy1=dy2; dy2=y;}
    if (sy2<sy1){y=sy1; sy1=sy2; sy2=y;}
    
    clip:
    //crop dest offscreen pixels
    if (dx1<0){
        if (mirror) sx2+=dx1; else sx1-=dx1;
        dx1=0;
    }
    if (dy1<0){
        if (flip) sy2+=dy1; else sy1-=dy1;
        dy1=0;
    }
    if (dx2>=dw){
        if (mirror) sx1+=(dx2-dw+1); else sx2-=(dx2-dw+1);
        dx2=dw-1;
    }
    if (dy2>=dh){
        if (flip) sy1+=(dy2-dh+1); else sy2-=(dy2-dh+1);
        dy2=dh-1;
    }
    //crop source offscreen pixels
    if (sx1<0){
        if (mirror) dx2+=sx1; else dx1-=sx1;
        sx1=0;
    }
    if (sy1<0){
        if (flip) dy2+=sy1; else dy1-=sy1;
        sy1=0;
    }
    if (sx2>=sw){
        if (mirror) dx1+=(sx2-sw+1); else dx2-=(sx2-sw+1);
        sx2=sw-1;
    }
    if (sy2>=sh){
        if (flip) dy1+=(sy2-sh+1); else dy2-=(sy2-sh+1);
        sy2=sh-1;
    }
    //<0-size/offscreen?
    //note: <0-size will cause reversal of dest
    //      offscreen values will result in reversal of dest
    if (dx1>dx2) return;
    if (dy1>dy2) return;
    //all values are now within the boundries of the source & dest
    
    //mirror put
    if (mirror){
        if (sbpp==4){
            if (s->alpha_disabled||d->alpha_disabled) goto put_32_noalpha_mirror;
            goto put_32_mirror;
        }
        if (dbpp==1){
            if (s->transparent_color==-1) goto put_8_mirror;
            goto put_8_clear_mirror;
        }
        if (s->transparent_color==-1) goto put_8_32_mirror;
        goto put_8_32_clear_mirror;
    }//mirror put
    
    noflip:
    if (sbpp==4){
        if (s->alpha_disabled||d->alpha_disabled) goto put_32_noalpha;
        goto put_32;
    }
    if (dbpp==1){
        if (s->transparent_color==-1) goto put_8;
        goto put_8_clear;
    }
    if (s->transparent_color==-1) goto put_8_32;
    goto put_8_32_clear;
    
    put_32:
    w=dx2-dx1+1;
    doff32=d->offset32+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff32=s->offset32+(sy2*sw+sx1);
        sskip=-w-sw;
        }else{
        soff32=s->offset32+(sy1*sw+sx1);
        sskip=sw-w;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            //--------plot pixel--------
            switch((col=*soff32++)&0xFF000000){
                case 0xFF000000:
                *doff32++=col;
                break;
                case 0x0:
                doff32++;
                break;
                case 0x80000000:
                *doff32++=(((*doff32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
                break; 
                case 0x7F000000:
                *doff32++=(((*doff32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
                break;
                default:
                destcol=*doff32;
                cp=cblend+(col>>24<<16);
                *doff32++=
                cp[(col<<8&0xFF00)+(destcol&255)    ]
                +(cp[(col&0xFF00)   +(destcol>>8&255) ]<<8)
                +(cp[(col>>8&0xFF00)+(destcol>>16&255)]<<16)
                +(ablend[(col>>24)+(destcol>>16&0xFF00)]<<24);
            };//switch
            //--------done plot pixel--------
        }while(--xx);
        soff32+=sskip; doff32+=dskip;
    }while(--h);
    return;
    
    put_32_noalpha:
    doff32=d->offset32+(dy1*dw+dx1);
    if (flip){
        soff32=s->offset32+(sy2*sw+sx1);
        sskip=-sw;
        }else{
        soff32=s->offset32+(sy1*sw+sx1);
        sskip=sw;
    }
    h=dy2-dy1+1;
    w=(dx2-dx1+1)*4;
    while(h--){
        memcpy(doff32,soff32,w);
        soff32+=sskip; doff32+=dw;
    }
    return;
    
    put_8:
    doff=d->offset+(dy1*dw+dx1);
    if (flip){
        soff=s->offset+(sy2*sw+sx1);
        sskip=-sw;
        }else{
        soff=s->offset+(sy1*sw+sx1);
        sskip=sw;
    }
    h=dy2-dy1+1;
    w=dx2-dx1+1;
    while(h--){
        memcpy(doff,soff,w);
        soff+=sskip; doff+=dw;
    }
    return;
    
    put_8_clear:
    clearcol=s->transparent_color;
    w=dx2-dx1+1;
    doff=d->offset+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff=s->offset+(sy2*sw+sx1);
        sskip=-w-sw;
        }else{
        soff=s->offset+(sy1*sw+sx1);
        sskip=sw-w;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            if ((col=*soff++)!=clearcol){
                *doff=col;
            }
            doff++;
        }while(--xx);
        soff+=sskip; doff+=dskip;
    }while(--h);
    return;
    
    put_8_32:
    pal=s->pal;
    w=dx2-dx1+1;
    doff32=d->offset32+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff=s->offset+(sy2*sw+sx1);
        sskip=-w-sw;
        }else{
        soff=s->offset+(sy1*sw+sx1);
        sskip=sw-w;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            *doff32++=pal[*soff++];
        }while(--xx);
        soff+=sskip; doff32+=dskip;
    }while(--h);
    return;
    
    put_8_32_clear:
    pal=s->pal;
    clearcol=s->transparent_color;
    w=dx2-dx1+1;
    doff32=d->offset32+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff=s->offset+(sy2*sw+sx1);
        sskip=-w-sw;
        }else{
        soff=s->offset+(sy1*sw+sx1);
        sskip=sw-w;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            if ((col=*soff++)!=clearcol){
                *doff32=pal[col];
            }
            doff32++;
        }while(--xx);
        soff+=sskip; doff32+=dskip;
    }while(--h);
    return;
    
    put_32_mirror:
    w=dx2-dx1+1;
    doff32=d->offset32+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff32=s->offset32+(sy2*sw+sx2);
        sskip=-sw+w;
        }else{
        soff32=s->offset32+(sy1*sw+sx2);
        sskip=w+sw;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            //--------plot pixel--------
            switch((col=*soff32--)&0xFF000000){
                case 0xFF000000:
                *doff32++=col;
                break;
                case 0x0:
                doff32++;
                break;
                case 0x80000000:
                *doff32++=(((*doff32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend128[*doff32>>24]<<24);
                break; 
                case 0x7F000000:
                *doff32++=(((*doff32&0xFEFEFE)+(col&0xFEFEFE))>>1)+(ablend127[*doff32>>24]<<24);
                break;
                default:
                destcol=*doff32;
                cp=cblend+(col>>24<<16);
                *doff32++=
                cp[(col<<8&0xFF00)+(destcol&255)    ]
                +(cp[(col&0xFF00)   +(destcol>>8&255) ]<<8)
                +(cp[(col>>8&0xFF00)+(destcol>>16&255)]<<16)
                +(ablend[(col>>24)+(destcol>>16&0xFF00)]<<24);
            };//switch
            //--------done plot pixel--------
        }while(--xx);
        soff32+=sskip; doff32+=dskip;
    }while(--h);
    return;
    
    put_32_noalpha_mirror:
    w=dx2-dx1+1;
    doff32=d->offset32+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff32=s->offset32+(sy2*sw+sx2);
        sskip=-sw+w;
        }else{
        soff32=s->offset32+(sy1*sw+sx2);
        sskip=w+sw;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            *doff32++=*soff32--;
        }while(--xx);
        soff32+=sskip; doff32+=dskip;
    }while(--h);
    return;
    
    put_8_mirror:
    w=dx2-dx1+1;
    doff=d->offset+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff=s->offset+(sy2*sw+sx2);
        sskip=-sw+w;
        }else{
        soff=s->offset+(sy1*sw+sx2);
        sskip=w+sw;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            *doff++=*soff--;
        }while(--xx);
        soff+=sskip; doff+=dskip;
    }while(--h);
    return;
    
    put_8_clear_mirror:
    clearcol=s->transparent_color;
    w=dx2-dx1+1;
    doff=d->offset+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff=s->offset+(sy2*sw+sx2);
        sskip=-sw+w;
        }else{
        soff=s->offset+(sy1*sw+sx2);
        sskip=w+sw;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            if ((col=*soff--)!=clearcol){
                *doff=col;
            }
            doff++;
        }while(--xx);
        soff+=sskip; doff+=dskip;
    }while(--h);
    return;
    
    put_8_32_mirror:
    pal=s->pal;
    w=dx2-dx1+1;
    doff32=d->offset32+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff=s->offset+(sy2*sw+sx2);
        sskip=-sw+w;
        }else{
        soff=s->offset+(sy1*sw+sx2);
        sskip=w+sw;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            *doff32++=pal[*soff--];
        }while(--xx);
        soff+=sskip; doff32+=dskip;
    }while(--h);
    return;
    
    put_8_32_clear_mirror:
    pal=s->pal;
    clearcol=s->transparent_color;
    w=dx2-dx1+1;
    doff32=d->offset32+(dy1*dw+dx1);
    dskip=dw-w;
    if (flip){
        soff=s->offset+(sy2*sw+sx2);
        sskip=-sw+w;
        }else{
        soff=s->offset+(sy1*sw+sx2);
        sskip=w+sw;
    }
    //plot rect
    h=dy2-dy1+1;
    do{
        xx=w;
        do{
            if ((col=*soff--)!=clearcol){
                *doff32=pal[col];
            }
            doff32++;
        }while(--xx);
        soff+=sskip; doff32+=dskip;
    }while(--h);
    return;
    
}//_putimage
