// Verbatim copy of legacy internal/c/libqb.cpp (oracle commit 545b859), lines 19992-20104 20236-20258 20904-21078.
// Test-only reference oracle for tests/diff/diff_gfx.cpp; production code is crates/qb-runtime/src/gfx.
        void sub__clearcolor(uint32 c,int32 i,int32 passed){
            //--         _NONE->1       2       4
            //id.specialformat = "[{_NONE}][?][,?]"
            if (new_error) return;
            static img_struct *im;
            static int32 z;
            static uint32 *lp,*last;
            static uint8 b_max,b_min,g_max,g_min,r_max,r_min;
            static uint8 *cp,*clast,v;
            if (passed&4){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            im=&img[i];
            //text?
            if (im->text){
                if ((passed&1)&&(!(passed&2))) return; //you can disable clearcolor using _CLEARCOLOR _NONE in text modes
                error(5); return;
            }
            //palette?
            if (im->pal){
                if (passed&1){
                    if (passed&2){error(5); return;}//invalid options
                    im->transparent_color=-1;
                    return;
                }
                if (!(passed&2)){error(5); return;}//invalid options
                if (c>255){error(5); return;}//invalid color
                im->transparent_color=c;
                return;
            }
            //32-bit? (alpha is ignored in this case)
            if (passed&1){
                if (passed&2){error(5); return;}//invalid options
                return;//no action
            }
            if (!(passed&2)){error(5); return;}//invalid options
            c&=0xFFFFFF;
            last=im->offset32+im->width*im->height;
            for (lp=im->offset32;lp<last;lp++){
                if ((*lp&0xFFFFFF)==c) *lp=c;
            }
            return;
        }
        
        //Changing/Using an image surface:
        
        //_PUT "[(?,?)[-(?,?)]][,[?][,[?][,[(?,?)[-(?,?)]]]]]"
        //(defined elsewhere)
        
        //_IMGALPHA "?[,[?[{TO}?]][,?]]"
        void sub__setalpha(int32 a,uint32 c,uint32 c2,int32 i,int32 passed){
            //-->                             1        4        2
            static img_struct *im;
            static int32 z;
            static uint32 *lp,*last;
            static uint8 b_max,b_min,g_max,g_min,r_max,r_min,a_max,a_min;
            static uint8 *cp,*clast,v;
            if (new_error) return;
            if (passed&2){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            im=&img[i];
            if (im->pal){error(5); return;}//does not work on paletted images!
            if (a<0||a>255){error(5); return;}//invalid range
            if (passed&4){
                //ranged
                if (c==c2) goto uniquerange;
                b_min=c&0xFF;  g_min=c>>8&0xFF;  r_min=c>>16&0xFF; a_min=c>>24&0xFF;
                b_max=c2&0xFF; g_max=c2>>8&0xFF; r_max=c2>>16&0xFF; a_max=c2>>24&0xFF;
                if (b_min>b_max) swap(b_min,b_max);
                
                if (g_min>g_max) swap(g_min,g_max);
                if (r_min>r_max) swap(r_min,r_max);
                if (a_min>a_max) swap(a_min,a_max);
                cp=im->offset;
                z=im->width*im->height;
                setalpha:
                if (z--){
                    v=*cp; if (v<=b_max&&v>=b_min){
                        v=*(cp+1); if (v<=g_max&&v>=g_min){
                            v=*(cp+2); if (v<=r_max&&v>=r_min){
                                v=*(cp+3); if (v<=a_max&&v>=a_min){
                                    *(cp+3)=a;
                                }}}}
                                cp+=4;
                                goto setalpha;
                }
                return;
            }
            if (passed&1){
                uniquerange:
                //alpha of c=a
                c2=a<<24;
                lp=im->offset32-1;
                last=im->offset32+im->width*im->height-1;
                while (lp<last){
                    if (*++lp==c){
                        *lp=(*lp&0xFFFFFF)|c2;
                    }
                }
                return;
            }
            //all alpha=a
            cp=im->offset-1;
            clast=im->offset+im->width*im->height*4-4;
            while (cp<clast){*(cp+=4)=a;}
            return;
        }
        
        uint32 func__palettecolor(int32 n,int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                }else{
                i=write_page_index;
            }
            if (!img[i].pal){error(5); return 0;}
            if (n<0||n>255){error(5); return 0;}//out of range
            return img[i].pal[n]|0xFF000000;
        }
        
        void sub__palettecolor(int32 n,uint32 c,int32 i,int32 passed){
            if (new_error) return;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return;
                }else{
                i=write_page_index;
            }
            if (!img[i].pal){error(5); return;}
            if (n<0||n>255){error(5); return;}//out of range
            img[i].pal[n]=c;
        }
        uint32 matchcol(int32 r,int32 g,int32 b){
            static int32 v,v2,n,n2,best,c;
            static int32 *p;
            p=(int32*)write_page->pal;
            if (write_page->text) n2=16; else n2=write_page->mask+1;
            v=1000;
            best=0;
            for (n=0;n<n2;n++){
                c=*p++;
                v2=abs(b-(c&0xFF))+abs(g-(c>>8&0xFF))+abs(r-(c>>16&0xFF));
                if (v2<v){
                    if (!v2) return n;//perfect match
                    v=v2;
                    best=n;
                }
            }//n
            return best;
        }
        
        uint32 matchcol(int32 r,int32 g,int32 b,int32 i){
            static int32 v,v2,n,n2,best,c;
            static int32 *p;
            p=(int32*)img[i].pal;
            if (img[i].text) n2=16; else n2=img[i].mask+1;
            v=1000;
            best=0;
            for (n=0;n<n2;n++){
                c=*p++;
                v2=abs(b-(c&0xFF))+abs(g-(c>>8&0xFF))+abs(r-(c>>16&0xFF));
                if (v2<v){
                    if (!v2) return n;//perfect match
                    v=v2;
                    best=n;
                }
            }//n
            return best;
        }
        
        uint32 func__rgb(int32 r,int32 g,int32 b,int32 i,int32 passed){
            if (new_error) return 0;
            if (r<0) r=0;
            if (r>255) r=255;
            if (g<0) g=0;
            if (g>255) g=255;
            if (b<0) b=0;
            if (b>255) b=255;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                if (img[i].bytes_per_pixel==4){
                    return (r<<16)+(g<<8)+b|0xFF000000;
                    }else{//==4
                    return matchcol(r,g,b,i);
                }//==4
                }else{
                if (write_page->bytes_per_pixel==4){
                    return (r<<16)+(g<<8)+b|0xFF000000;
                    }else{//==4
                    return matchcol(r,g,b);
                }//==4
            }//passed
        }//rgb
        
        uint32 func__rgba(int32 r,int32 g,int32 b,int32 a,int32 i,int32 passed){
            if (new_error) return 0;
            if (r<0) r=0;
            if (r>255) r=255;
            if (g<0) g=0;
            if (g>255) g=255;
            if (b<0) b=0;
            if (b>255) b=255;
            if (a<0) a=0;
            if (a>255) a=255;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                if (img[i].bytes_per_pixel==4){
                    return (a<<24)+(r<<16)+(g<<8)+b;
                    }else{//==4
                    //error(5); return 0;
                    if ((!a)&&(img[i].transparent_color!=-1)) return img[i].transparent_color;
                    return matchcol(r,g,b,i);
                }//==4
                }else{
                if (write_page->bytes_per_pixel==4){
                    return (a<<24)+(r<<16)+(g<<8)+b;
                    }else{//==4
                    //error(5); return 0;
                    if ((!a)&&(write_page->transparent_color!=-1)) return write_page->transparent_color;
                    return matchcol(r,g,b);
                }//==4
            }//passed
        }//rgba
        
        int32 func__alpha(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                if (img[i].bytes_per_pixel==4){
                    return col>>24;
                    }else{//==4
                    //error(5); return 0; 
                    if ((col<0)||(col>(img[i].mask))){error(5); return 0;} 
                    if (img[i].transparent_color==col) return 0;
                    return 255;
                }//==4
                }else{
                if (write_page->bytes_per_pixel==4){
                    return col>>24;
                    }else{//==4
                    //error(5); return 0; 
                    if ((col<0)||(col>(write_page->mask))){error(5); return 0;} 
                    if (write_page->transparent_color==col) return 0;
                    return 255;
                }//==4
            }//passed
        }
        
        int32 func__red(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                if (img[i].bytes_per_pixel==4){
                    return col>>16&0xFF;
                    }else{//==4
                    if ((col<0)||(col>(img[i].mask))){error(5); return 0;}
                    return img[i].pal[col]>>16&0xFF;
                }//==4
                }else{
                if (write_page->bytes_per_pixel==4){
                    return col>>16&0xFF;
                    }else{//==4
                    if ((col<0)||(col>(write_page->mask))){error(5); return 0;}
                    return write_page->pal[col]>>16&0xFF;
                }//==4
            }//passed
        }
        
        int32 func__green(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                if (img[i].bytes_per_pixel==4){
                    return col>>8&0xFF;
                    }else{//==4
                    if ((col<0)||(col>(img[i].mask))){error(5); return 0;}
                    return img[i].pal[col]>>8&0xFF;
                }//==4
                }else{
                if (write_page->bytes_per_pixel==4){
                    return col>>8&0xFF;
                    }else{//==4
                    if ((col<0)||(col>(write_page->mask))){error(5); return 0;}
                    return write_page->pal[col]>>8&0xFF;
                }//==4
            }//passed
        }
        
        int32 func__blue(uint32 col,int32 i,int32 passed){
            if (new_error) return 0;
            if (passed){
                if (!resolve_page_or_image_index(i,i)) return 0;
                if (img[i].bytes_per_pixel==4){
                    return col&0xFF;
                    }else{//==4
                    if ((col<0)||(col>(img[i].mask))){error(5); return 0;}
                    return img[i].pal[col]&0xFF;
                }//==4
                }else{
                if (write_page->bytes_per_pixel==4){
                    return col&0xFF;
                    }else{//==4
                    if ((col<0)||(col>(write_page->mask))){error(5); return 0;}
                    return write_page->pal[col]&0xFF;
                }//==4
            }//passed
        }
