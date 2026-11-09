// Verbatim copy of legacy internal/c/libqb.cpp (oracle commit 545b859), lines 22120-22561.
// Test-only reference oracle for tests/diff_gfx.cpp; production code is internal/c/qbrt/src/gfx/draw.rs.
        int32 sub_draw_i;
        uint8 *sub_draw_cp;
        int32 sub_draw_len;
        
        int32 draw_num_invalid;
        int32 draw_num_undefined;
        double draw_num(){
            static int32 c,dp,vptr,x,offset;
            static double d,dp_mult,sgn;
            
            draw_num_invalid=0;
            draw_num_undefined=1;
            d=0;
            dp=0;
            sgn=1;
            vptr=0;
            
            nextchar:
            if (sub_draw_i>=sub_draw_len) return d*sgn;
            c=sub_draw_cp[sub_draw_i];
            
            if (vptr){
                if ((sub_draw_i+2)>=sub_draw_len) {draw_num_invalid=1; return 0;}//not enough data!
                offset=sub_draw_cp[sub_draw_i+2]*256+sub_draw_cp[sub_draw_i+1];
                sub_draw_i+=3;
                vptr=0;
                /*
                    'BYTE=1
                    'INTEGER=2
                    'STRING=3 (unsupported)
                    'SINGLE=4
                    'INT64=5
                    'FLOAT=6
                    'DOUBLE=8
                    'LONG=20
                    'BIT=64+n (unsupported)
                */
                if (c==1){d=*((int8*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==(1+128)){d=*((uint8*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==2){d=*((int16*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==(2+128)){d=*((uint16*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==4){d=*((float*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==5){d=*((int64*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==(5+128)){d=*((uint64*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==6){d=*((long double*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==8){d=*((double*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==20){d=*((int32*)(&cmem[1280+offset])); goto nextcharv;}
                if (c==(20+128)){d=*((uint32*)(&cmem[1280+offset])); goto nextcharv;}
                //unknown/unsupported types(bit/string) return an error
                draw_num_invalid=1; return 0;
                nextcharv:
                draw_num_invalid=0;
                draw_num_undefined=0;
                return d;
            }
            
            if ((c==32)||(c==9)){sub_draw_i++; goto nextchar;}//skip whitespace
            
            if ((c>=48)&&(c<=57)){
                c-=48;
                if (dp){
                    d+=(((double)c)*dp_mult);
                    dp_mult/=10.0;
                    }else{
                    d=(d*10)+c;
                }
                draw_num_undefined=0;
                draw_num_invalid=0;
                sub_draw_i++; goto nextchar;
            }
            
            if (c==45){//-
                if (dp||(!draw_num_undefined)) return d*sgn;
                sgn=-sgn;
                draw_num_invalid=1;
                sub_draw_i++; goto nextchar;
            }
            
            if (c==43){//+
                if (dp||(!draw_num_undefined)) return d*sgn;
                draw_num_invalid=1;
                sub_draw_i++; goto nextchar;
            }
            
            if (c==46){//.
                if (dp) return d*sgn;
                dp=1; dp_mult=0.1;
                if (!draw_num_undefined) draw_num_invalid=1;
                sub_draw_i++; goto nextchar;
            }
            
            if (c==61){//=
                if (draw_num_invalid||dp||(!draw_num_undefined)){draw_num_invalid=1; return 0;}//leading data invalid!
                vptr=1;
                sub_draw_i++; goto nextchar;
            }
            
            return d*sgn;
        }
        
        void sub_draw(qbs* s){
            if (new_error) return;
            
            /*
                
                Aspect ratio determination:
                32/256 modes always assume 1:1 ratio
                All other modes (1-13) determine their aspect ratio from the destination surface's dimensions (presuming it is stretched onto a 4:3 ratio monitor)
                
                Reference:
                Line-drawing and cursor-movement commands:
                D[n%]            Moves cursor down n% units.
                E[n%]            Moves cursor up and right n% units.
                F[n%]            Moves cursor down and right n% units.
                G[n%]            Moves cursor down and left n% units.
                H[n%]            Moves cursor up and left n% units.
                L[n%]            Moves cursor left n% units.
                M[{+|-}]x%,y%    Moves cursor to point x%,y%. If x% is preceded
                by + or -, moves relative to the current point.
                -+/- relative ONLY if after the M, after comma doesn't affect method
                -nothing to do with VIEW/WINDOW coordinates (but still clipped)
                R[n%]            Moves cursor right n% units.
                U[n%]            Moves cursor up n% units.
                [B]              Optional prefix that moves cursor without drawing.
                [N]              Optional prefix that draws and returns cursor to
                its original position.
                *Prefixes B&N can be used anywhere. They set (not toggle) their respective states. They are only cleared if they are used in a statement. They are forgotten when a new DRAW statement is called.
                Color, rotation, and scale commands:
                An%              Rotates an object n% * 90 degrees (n% can be 0, 1,
                2, or 3).
                Cn%              Sets the drawing color (n% is a color attribute).
                
                Pn1%,n2%         Sets the paint fill and border colors of an object
                (n1% is the fill-color attribute, n2% is the
                border-color attribute).
                Sn%              Determines the drawing scale by setting the length
                of a unit of cursor movement. The default n% is 4,
                which is equivalent to 1 pixel.
                TAn%             Turns an angle n% degrees (-360 through 360).
                
                -If you omit n% from line-drawing and cursor-movement commands, the
                cursor moves 1 unit.
                -To execute a DRAW command substring from a DRAW command string, use
                the "X" command:
                DRAW "X"+ VARPTR$(commandstring$)
            */
            
            static double r,ir,vx,vy,hx,hy,ex,ey,fx,fy,xx,yy,px,py,px2,py2,d,d2,sin_ta,cos_ta;
            static int64 c64,c64b,c64c;
            static uint32 col;
            static int32 x,c,prefix_b,prefix_n,offset;
            static uint8 *stack_s[8192];
            static uint16 stack_len[8192];
            static uint16 stack_i[8192];
            static int32 stacksize;
            static double draw_ta;
            static double draw_scale;
            
            if (write_page->text){error(5); return;}
            
            draw_ta=write_page->draw_ta; draw_scale=write_page->draw_scale;
            
            if (write_page->compatible_mode<=13){
                if (write_page->compatible_mode==1) r=4.0/((3.0/200.0)*320.0);
                if (write_page->compatible_mode==2) r=4.0/((3.0/200.0)*640.0);
                if (write_page->compatible_mode==7) r=4.0/((3.0/200.0)*320.0);
                if (write_page->compatible_mode==8) r=4.0/((3.0/200.0)*640.0);
                if (write_page->compatible_mode==9) r=4.0/((3.0/350.0)*640.0);
                if (write_page->compatible_mode==10) r=4.0/((3.0/350.0)*640.0);
                if (write_page->compatible_mode==11) r=4.0/((3.0/480.0)*640.0); 
                if (write_page->compatible_mode==12) r=4.0/((3.0/480.0)*640.0);
                if (write_page->compatible_mode==13) r=4.0/((3.0/200.0)*320.0);
                //Old method: r=4.0 /( (3.0/((double)write_page->height)) * ((double)write_page->width) ); //calculate aspect ratio of image
                ir=1/r; //note: all drawing must multiply the x offset by ir (inverse ratio)
                }else{
                r=1;
                ir=1;
            }
            
            
            
            vx=0; vy=-1; ex=r; ey=-1; hx=r; hy=0; fx=r; fy=1;//reset vectors
            //rotate vectors by ta?
            if (draw_ta){
                d=draw_ta*0.0174532925199433; sin_ta=sin(d); cos_ta=cos(d);
                px2=vx;
                py2=vy;
                vx=px2*cos_ta+py2*sin_ta;
                vy=py2*cos_ta-px2*sin_ta;
                px2=hx;
                py2=hy;
                hx=px2*cos_ta+py2*sin_ta;
                hy=py2*cos_ta-px2*sin_ta;
                px2=ex;
                py2=ey;
                ex=px2*cos_ta+py2*sin_ta;
                ey=py2*cos_ta-px2*sin_ta;
                px2=fx;
                py2=fy;
                fx=px2*cos_ta+py2*sin_ta;
                fy=py2*cos_ta-px2*sin_ta;
            }
            
            //convert x,y image position into a pixel coordinate
            if (write_page->clipping_or_scaling){
                if (write_page->clipping_or_scaling==2){
                    px=write_page->x*write_page->scaling_x+write_page->scaling_offset_x+write_page->view_offset_x;
                    py=write_page->y*write_page->scaling_y+write_page->scaling_offset_y+write_page->view_offset_y;
                    }else{
                    px=write_page->x+write_page->view_offset_x;
                    py=write_page->y+write_page->view_offset_y;
                }
                }else{
                px=write_page->x;
                py=write_page->y;
            }
            
            col=write_page->draw_color;
            prefix_b=0; prefix_n=0;
            
            stacksize=0;
            
            sub_draw_cp=s->chr;
            sub_draw_len=s->len;
            sub_draw_i=0;
            
            nextchar:
            if (sub_draw_i>=sub_draw_len){
                
                //revert from X-stack
                if (stacksize){
                    stacksize--; sub_draw_cp=stack_s[stacksize]; sub_draw_len=stack_len[stacksize]; sub_draw_i=stack_i[stacksize];//restore state
                    //continue
                    goto nextchar;
                }
                
                //revert px,py to image->x,y offsets
                if (write_page->clipping_or_scaling){
                    if (write_page->clipping_or_scaling==2){
                        px=(px-write_page->view_offset_x-write_page->scaling_offset_x)/write_page->scaling_x;
                        py=(py-write_page->view_offset_y-write_page->scaling_offset_y)/write_page->scaling_y;
                        }else{
                        px=px-write_page->view_offset_x;
                        py=py-write_page->view_offset_y;
                    }
                }
                write_page->x=px; write_page->y=py;
                return;
            }
            c=sub_draw_cp[sub_draw_i];
            
            if ((c>=97)&&(c<=122)) c-=32;//ucase c
            
            if (c==77){//M
                m_nextchar:
                sub_draw_i++; if (sub_draw_i>=sub_draw_len){error(5); return;}
                c=sub_draw_cp[sub_draw_i];
                if ((c==32)||(c==9)) goto m_nextchar;//skip whitespace
                //check for absolute/relative positioning
                if ((c==43)||(c==45)) x=1; else x=0;
                px2=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                c=sub_draw_cp[sub_draw_i];
                if (c!=44){error(5); return;}//expected ,
                sub_draw_i++;
                py2=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                if (x){//relative positioning
                    xx=(px2*ir)*hx-(py2*ir)*vx; yy=px2*hy-py2*vy; px2=px+xx*draw_scale; py2=py+yy*draw_scale;
                }
                if (!prefix_b) fast_line(qbr(px),qbr(py),qbr(px2),qbr(py2),col);
                if (!prefix_n){px=px2; py=py2;}//update position
                prefix_b=0; prefix_n=0;
                goto nextchar;
            }
            
            if (c==84){//T(A)
                ta_nextchar:
                sub_draw_i++; if (sub_draw_i>=sub_draw_len){error(5); return;}
                c=sub_draw_cp[sub_draw_i];
                if ((c==32)||(c==9)) goto ta_nextchar;//skip whitespace
                if ((c!=65)&&(c!=97)){error(5); return;}//not TA
                sub_draw_i++;
                d=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                draw_ta=d;
                write_page->draw_ta=draw_ta;
                ta_entry:
                //note: ta rotation is not relative to previous angle
                vx=0; vy=-1; ex=r; ey=-1; hx=r; hy=0; fx=r; fy=1;//reset vectors
                //rotate vectors by ta
                d=draw_ta*0.0174532925199433; sin_ta=sin(d); cos_ta=cos(d);
                px2=vx;
                py2=vy;
                vx=px2*cos_ta+py2*sin_ta;
                vy=py2*cos_ta-px2*sin_ta;
                px2=hx;
                py2=hy;
                hx=px2*cos_ta+py2*sin_ta;
                hy=py2*cos_ta-px2*sin_ta;
                px2=ex;
                py2=ey;
                ex=px2*cos_ta+py2*sin_ta;
                ey=py2*cos_ta-px2*sin_ta;
                px2=fx;
                py2=fy;
                fx=px2*cos_ta+py2*sin_ta;
                fy=py2*cos_ta-px2*sin_ta;
                goto nextchar;
            }
            
            if (c==85){xx=vx; yy=vy; goto udlr;}//U
            if (c==68){xx=-vx; yy=-vy; goto udlr;}//D
            if (c==76){xx=-hx; yy=-hy; goto udlr;}//L
            if (c==82){xx=hx; yy=hy; goto udlr;}//R
            
            if (c==69){xx=ex; yy=ey; goto udlr;}//E
            if (c==70){xx=fx; yy=fy; goto udlr;}//F
            if (c==71){xx=-ex; yy=-ey; goto udlr;}//G
            if (c==72){xx=-fx; yy=-fy; goto udlr;}//H
            
            if (c==67){//C
                sub_draw_i++;
                d=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                c64=d; xx=c64; if (xx!=d){error(5); return;}//non-integer
                //if (c64<0){error(5); return;}
                //c64b=1; c64b<<=write_page->bits_per_pixel; c64b--;
                //if (c64>c64b){error(5); return;}
                col=c64;
                write_page->draw_color=col;
                goto nextchar;
            }
            
            if (c==66){//B (move without drawing prefix)
                prefix_b=1;
                sub_draw_i++;
                goto nextchar;
            }
            
            if (c==78){//N (draw without moving)
                prefix_n=1;
                sub_draw_i++;
                goto nextchar;
            }
            
            if (c==83){//S
                sub_draw_i++;
                d=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                if (d<0){error(5); return;}
                draw_scale=d/4.0;
                write_page->draw_scale=draw_scale;
                goto nextchar;
            }
            
            if (c==80){//P
                sub_draw_i++;
                d=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                c64=d; xx=c64; if (xx!=d){error(5); return;}//non-integer
                //if (c64<0){error(5); return;}
                //c64b=1; c64b<<=write_page->bits_per_pixel; c64b--;
                //if (c64>c64b){error(5); return;}
                c64c=c64;
                c=sub_draw_cp[sub_draw_i];
                if (c!=44){error(5); return;}//expected ,
                sub_draw_i++;
                d=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                c64=d; xx=c64; if (xx!=d){error(5); return;}//non-integer
                //if (c64<0){error(5); return;}
                //c64b=1; c64b<<=write_page->bits_per_pixel; c64b--;
                //if (c64>c64b){error(5); return;}
                //revert px,py to x,y offsets
                if (write_page->clipping_or_scaling){
                    if (write_page->clipping_or_scaling==2){
                        xx=(px-write_page->view_offset_x-write_page->scaling_offset_x)/write_page->scaling_x;
                        yy=(py-write_page->view_offset_y-write_page->scaling_offset_y)/write_page->scaling_y;
                        }else{
                        xx=px-write_page->view_offset_x;
                        yy=py-write_page->view_offset_y;
                    }
                    }else{
                    xx=px;
                    yy=py;
                }
                sub_paint(xx,yy,c64c,c64,NULL,2+4);
                col=c64c;
                goto nextchar;
            }
            
            if (c==65){//A
                sub_draw_i++;
                d=draw_num();
                if (draw_num_invalid||draw_num_undefined){error(5); return;}
                if (d==0){draw_ta=0; write_page->draw_ta=draw_ta; goto ta_entry;}
                if (d==1){draw_ta=90; write_page->draw_ta=draw_ta; goto ta_entry;}
                if (d==2){draw_ta=180; write_page->draw_ta=draw_ta; goto ta_entry;}
                if (d==3){draw_ta=270; write_page->draw_ta=draw_ta; goto ta_entry;}
                error(5); return;//invalid value
            }
            
            if (c==88){//X
                sub_draw_i++;
                if ((sub_draw_i+2)>=sub_draw_len){error(5); return;}
                if (sub_draw_cp[sub_draw_i]!=3){error(5); return;}
                offset=sub_draw_cp[sub_draw_i+2]*256+sub_draw_cp[sub_draw_i+1];//offset of string descriptor in DBLOCK
                sub_draw_i+=3;
                if (stacksize==8192){error(6); return;}//X-stack "OVERFLOW" (should never occur because DBLOCK will overflow first)
                stack_s[stacksize]=sub_draw_cp; stack_len[stacksize]=sub_draw_len; stack_i[stacksize]=sub_draw_i; stacksize++;//backup state
                //set new state
                sub_draw_i=0;
                x=cmem[1280+offset+3]*256+cmem[1280+offset+2];
                sub_draw_cp=&cmem[1280]+x;
                sub_draw_len=cmem[1280+offset+1]*256+cmem[1280+offset+0];
                //continue processing
                goto nextchar;
            }
            
            if ((c==32)||(c==9)||(c==59)){sub_draw_i++; goto nextchar;}//skip whitespace/semicolons
            
            error(5); return;//unknown command encountered!
            
            
            
            
            udlr:
            sub_draw_i++;
            d=draw_num();
            if (draw_num_invalid){error(5); return;}
            if (draw_num_undefined) d=1;
            xx*=d; yy*=d;
            //***apply scaling here***
            xx=xx*ir;
            px2=px+xx*draw_scale; py2=py+yy*draw_scale;
            if (!prefix_b) fast_line(qbr(px),qbr(py),qbr(px2),qbr(py2),col);
            if (!prefix_n){px=px2; py=py2;}//update position
            prefix_b=0; prefix_n=0;
            goto nextchar;
        }
        
