// Verbatim fragments of legacy qbg_screen (libqb.cpp) for diff_gfx (test-only).
// The surrounding stubs record what the fragments ask for instead of building surfaces.
namespace legacy_screen {
static int32 sub_screen_width_in_characters=-1, sub_screen_height_in_characters=-1, sub_screen_font=-1;
static int32 rec_w, rec_h, rec_mode, rec_font, rec_frame;
static uint8 cmem_stub[655361];
struct stub_img { int32 flags; };
static stub_img img[4];
static int32 page[1];
static int32 imgnew(int32 w,int32 h,int32 m){ rec_w=w; rec_h=h; rec_mode=m; rec_frame=0; return 1; }
static int32 imgframe(uint8 *,int32 w,int32 h,int32 m){ rec_w=w; rec_h=h; rec_mode=m; rec_frame=1; return 1; }
static void selectfont(int32 f,stub_img *){ rec_font=f; }
static void select_all_pages(int32){}
#define IMG_SCREEN 2

// returns 1 when `SCREEN mode` passes the legacy mode check (mode>=0)
static int32 mode_valid(int32 mode){
            if (mode==3) goto error;
            if (mode==4) goto error;
            if (mode==5) goto error;
            if (mode==6) goto error;
            if (mode>13) goto error;
    return 1;
    error:
    return 0;
}

// graphics modes: records the surface the legacy code would create
static int32 graphics(int32 mode,int32 prev_width_in_characters,int32 prev_height_in_characters){
    static int32 i;
    rec_w=0; rec_h=0; rec_mode=-1; rec_font=0; rec_frame=0;
#define memset(a,b,c) ((void)0)
#define cmem cmem_stub
            if (mode==13){
                i=imgframe(&cmem[655360],320,200,13);
                memset(img[i].offset,0,320*200);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//13
            
            //640 x 480 graphics
            //80 x 30 or 80 x 60 text format, character box size of 8 x 16 or 8 x 8
            //Assignment of up to 256K colors to 16 attributes
            if (mode==12){
                i=imgnew(640,480,12);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==60)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//12
            
            /*
                Screen 11
                \A6 640 x 480 graphics
                \A6 80 x 30 or 80 x 60 text format, character box size of 8 x 16 or 8 x 8
                \A6 Assignment of up to 256K colors to 2 attributes
            */
            if (mode==11){
                i=imgnew(640,480,11);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==60)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//11
            
            //SCREEN 10: 640 x 350 graphics, monochrome monitor only
            //  \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
            //  \A6 128K page size, page range is 0 (128K) or 0-1 (256K)
            //  \A6 Up to 9 pseudocolors assigned to 4 attributes
            /*
                'colors swap every half second!
                'using PALETTE does NOT swap color indexes
                '0 black-black
                '1 black-grey
                '2 black-white
                '3 grey-black
                '4 grey-grey
                '5 grey-white
                '6 white-black
                '7 white-grey
                '8 white-white
                '*IMPORTANT* QB sets initial values up different to default palette!
                '0 block-black(0)
                '1 grey-grey(4)
                '2 white-black(6)
                '3 white-white(8)
            */
            if (mode==10){
                i=imgnew(640,350,10);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==43)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
                
            }//10
            
            /*
                SCREEN 9: 640 x 350 graphics
                \A6 80 x 25 or 80 x 43 text format, 8 x 14 or 8 x 8 character box size
                \A6 64K page size, page range is 0 (64K);
                128K page size, page range is 0 (128K) or 0-1 (256K)
                \A6 16 colors assigned to 4 attributes (64K adapter memory), or
                64 colors assigned to 16 attributes (more than 64K adapter memory)
            */
            if (mode==9){
                i=imgnew(640,350,9);
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==43)) selectfont(8,&img[i]);//override default font
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//9
            
            /*
                SCREEN 8: 640 x 200 graphics
                \A6 80 x 25 text format, 8 x 8 character box
                \A6 64K page size, page ranges are 0 (64K), 0-1 (128K), or 0-3 (246K)
                \A6 Assignment of 16 colors to any of 16 attributes
            */
            if (mode==8){
                i=imgnew(640,200,8);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//8
            
            /*
                SCREEN 7: 320 x 200 graphics
                \A6 40 x 25 text format, character box size 8 x 8
                \A6 32K page size, page ranges are 0-1 (64K), 0-3 (128K), or 0-7 (256K)
                \A6 Assignment of 16 colors to any of 16 attributes
            */
            if (mode==7){
                i=imgnew(320,200,7);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//7
            
            /*
                SCREEN 4:
                \A6 Supports Olivetti (R) Personal Computers models M24, M240, M28,
                M280, M380, M380/C, M380/T and AT&T (R) Personal Computers 6300
                series
                \A6 640 x 400 graphics
                \A6 80 x 25 text format, 8 x 16 character box
                \A6 1 of 16 colors assigned as the foreground color (selected by the
                COLOR statement); background is fixed at black.
            */
            //Note: QBNex will not support SCREEN 4
            
            /*
                SCREEN 3: Hercules adapter required, monochrome monitor only
                \A6 720 x 348 graphics
                \A6 80 x 25 text format, 9 x 14 character box
                \A6 2 screen pages (1 only if a second display adapter is installed)
                \A6 PALETTE statement not supported
            */
            //Note: QBNex will not support SCREEN 3
            
            /*
                SCREEN 2: 640 x 200 graphics
                \A6 80 x 25 text format with character box size of 8 x 8
                \A6 16 colors assigned to 2 attributes with EGA or VGA
            */
            if (mode==2){
                i=imgnew(640,200,2);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//2
            
            /*
                SCREEN 1: 320 x 200 graphics
                \A6 40 x 25 text format, 8 x 8 character box
                \A6 16 background colors and one of two sets of 3 foreground colors assigned
                using COLOR statement with CGA
                \A6 16 colors assigned to 4 attributes with EGA or VGA
            */
            if (mode==1){
                i=imgnew(320,200,1);
                page[0]=i; img[i].flags|=IMG_SCREEN; select_all_pages(i);
            }//1
#undef memset
#undef cmem
    return rec_mode!=-1;
}

// SCREEN 0 layout: out = {x, y, f, granularity, pages}
static void screen0(int32 prev_width_in_characters,int32 prev_height_in_characters,int32 *out){
    static int32 x,y,f,i2,p;
                if ((sub_screen_width_in_characters!=-1)&&(sub_screen_height_in_characters!=-1)&&(sub_screen_font!=-1)){
                    x=sub_screen_width_in_characters; y=sub_screen_height_in_characters; f=sub_screen_font;
                    sub_screen_width_in_characters=-1; sub_screen_height_in_characters=-1; sub_screen_font=-1;
                    goto gotwidth;
                }
                if (sub_screen_width_in_characters!=-1){
                    x=sub_screen_width_in_characters; sub_screen_width_in_characters=-1;
                    y=25; f=16;//default
                    if (prev_height_in_characters==43){y=43; f=14;}
                    if (prev_height_in_characters==50){y=50; f=8;}
                    if (x==40) f++;
                    goto gotwidth;
                }
                if (sub_screen_height_in_characters!=-1){
                    y=sub_screen_height_in_characters; sub_screen_height_in_characters=-1;
                    f=16;//default
                    if (y==43) f=14;
                    if (y==50) f=8;
                    x=80;//default
                    if (prev_width_in_characters==40){f++; x=40;}
                    goto gotwidth;
                }
                
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==50)){
                    x=80; y=50; f=8; goto gotwidth;
                }
                if ((prev_width_in_characters==40)&&(prev_height_in_characters==50)){
                    x=40; y=50; f=8+1; goto gotwidth;
                }
                if ((prev_width_in_characters==80)&&(prev_height_in_characters==43)){
                    x=80; y=43; f=8; goto gotwidth;
                }
                if ((prev_width_in_characters==40)&&(prev_height_in_characters==43)){
                    x=40; y=43; f=8+1; goto gotwidth;
                }
                if ((prev_width_in_characters==40)&&(prev_height_in_characters==25)){
                    x=40; y=25; f=16+1; goto gotwidth;
                }
                x=80; y=25; f=16;
                gotwidth:;
                i2=x*y*2;//default granularity
                //specific granularities which cannot be calculated
                if ((x==40)&&(y==25)&&(f=(16+1))) i2=2048;
                if ((x==80)&&(y==25)&&(f=16)) i2=4096;
                p=65536/i2;//number of pages to allocate in cmem
                if (p>8) p=8;//limit cmem pages to 8
    out[0]=x; out[1]=y; out[2]=f; out[3]=i2; out[4]=p;
}
} // namespace legacy_screen
