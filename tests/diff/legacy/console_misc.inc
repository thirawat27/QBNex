// Verbatim from internal/c/libqb.cpp (R12a): INKEY$, _KEYCLEAR, CSRLIN, POS, SCREEN(), INPUT$ (keyboard).
qbs *qbs_inkey(){
    if (new_error) return qbs_new(0,1);
    qbs *tqbs;
    // Sleep(0);
    tqbs=qbs_new(2,1);
    if (cmem[0x41a]!=cmem[0x41c]){
        tqbs->chr[0]=cmem[0x400+cmem[0x41a]];
        tqbs->chr[1]=cmem[0x400+cmem[0x41a]+1];
        if (tqbs->chr[0]){
            tqbs->len=1;
            }else{
            if (tqbs->chr[1]==0) tqbs->len=1;
        }
        cmem[0x41a]+=2;
        if (cmem[0x41a]==62) cmem[0x41a]=30;
        }else{
        tqbs->len=0;
    }
    return tqbs;
}

void sub__keyclear(int32 buf, int32 passed) {
    if (new_error) return;
    if (passed && (buf > 3 || buf < 1)) error(5);
    //  Sleep(10);
    if ((buf == 1 && passed) || !passed) {
        //INKEY$ buffer
        cmem[0x41a]=30; cmem[0x41b]=0; //head
        cmem[0x41c]=30; cmem[0x41d]=0; //tail
    }
    if ((buf == 2 && passed) || !passed) {
        //_KEYHIT buffer
        qb_keyhit_clear();
    }
    if ((buf == 3 && passed) || !passed) {
        //INP(&H60) buffer
        port60h_events = 0;
    }
    #ifdef QBNex_WINDOWS
        //Windows Console Buffer
        FlushConsoleInputBuffer(GetStdHandle (STD_INPUT_HANDLE));
    #endif


}
    int32 func_csrlin(){
        #ifdef QBNex_WINDOWS //Windows console CSRLIN support
            if (write_page->console){
                CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
                SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                GetConsoleScreenBufferInfo(cl_conout, & cl_bufinfo);
                return cl_bufinfo.dwCursorPosition.Y + 1;
            }
        #endif
        if (write_page->holding_cursor){
            if (write_page->cursor_y>=write_page->bottom_row) return write_page->bottom_row; else return write_page->cursor_y+1;
        }
        return write_page->cursor_y;
    }
    int32 func_pos(int32 ignore){
        #ifdef QBNex_WINDOWS
            if (write_page->console){ //qbnex console CSRLIN 
                CONSOLE_SCREEN_BUFFER_INFO cl_bufinfo;
                SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                GetConsoleScreenBufferInfo(cl_conout, & cl_bufinfo);
                return cl_bufinfo.dwCursorPosition.X + 1;
            }
        #endif
        if (write_page->holding_cursor) return 1;
        return write_page->cursor_x;
    }
    uint32 func_screen(int32 y,int32 x,int32 returncol,int32 passed){
        
        static uint8 chr[65536];
        static int32 x2,y2,x3,y3,i,i2,i3;
        static uint32 col,firstcol;
        uint8 *cp;
        
        if (!passed) returncol=0;

        #ifdef QBNex_WINDOWS
            if (read_page->console){
                SECURITY_ATTRIBUTES SecAttribs = {sizeof(SECURITY_ATTRIBUTES), 0, 1};
                HANDLE cl_conout = CreateFileA("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, & SecAttribs, OPEN_EXISTING, 0, 0);
                COORD cp1 = {x-1, y-1};
                DWORD t;
                uint16 a; 
                if (returncol){
                    int x1 = ReadConsoleOutputAttribute(cl_conout, &a, 1, cp1, &t) ;
                    return a;
                } else {
                    ReadConsoleOutputCharacterA(cl_conout, (char *) &a, 1, cp1, &t) ;
                    return a & 0xff;
                }
            }
        #endif
        
        if (read_page->text){
            //on screen?
            if ((y<1)||(y>read_page->height)){error(5); return 0;}
            if ((x<1)||(x>read_page->width)){error(5); return 0;}
            if (returncol){
                return read_page->offset[((y-1)*read_page->width+x-1)*2+1];
            }
            return read_page->offset[((y-1)*read_page->width+x-1)*2];
        }
        
        //only 8x8,8x14,8x16 supported
        if ((read_page->font!=8)&&(read_page->font!=14)&&(read_page->font!=16)){error(5); return 0;}
        
        //on screen?
        x2=read_page->width/fontwidth[read_page->font];
        y2=read_page->height/fontheight[read_page->font];
        if ((y<1)||(y>y2)){error(5); return 0;}
        if ((x<1)||(x>x2)){error(5); return 0;}
        
        //create "signature" of character on screen
        x--; y--;
        i=0;
        i3=1;
        y3=y*fontheight[read_page->font];
        for (y2=0;y2<fontheight[read_page->font];y2++){
            x3=x*fontwidth[read_page->font];
            for (x2=0;x2<fontwidth[read_page->font];x2++){
                col=point(x3,y3);
                if (col){
                    if (i3){i3=0;firstcol=col;}
                    col=255;
                }
                chr[i]=col;
                i++;
                x3++;
            }
            y3++;
        }
        
        if (i3){//assume SPACE, no non-zero pixels were found
            if (returncol) return 1;
            return 32;
        }
        
        i3=i;//number of bytes per character "signature"
        
        //compare signature with all ascii characters
        for (i=0;i<=255;i++){
            if (read_page->font==8) cp=&charset8x8[i][0][0];
            if (read_page->font==14) cp=&charset8x16[i][1][0];
            if (read_page->font==16) cp=&charset8x16[i][0][0];
            i2=memcmp(cp,chr,i3);
            if (!i2){//identical!
                if (returncol) return firstcol;
                return i;
            }
        }
        
        //no match found
        if (returncol) return 0;
        return 32;
    }
    qbs *func_input(int32 n,int32 i,int32 passed){
        if (new_error) return qbs_new(0,1);
        static qbs *str,*str2;
        static int32 x;
        if (n<0) str=qbs_new(0,1); else str=qbs_new(n,1);
        if (passed){
            int32 len=str->len;
            int32 e=qb_file_input_n(n,i,str->chr,str->len,&len);
            str->len=len;
            if (e) error(e);
            return str;
            }else{
            //keyboard/piped
            //      For extended-two-byte character codes, only the first, CHR$(0), is returned and counts a 1 byte
            if (n<0){error(52); return str;}
            if (n==0) return str;
            x=0;
            waitforinput:
            str2=qbs_inkey();
            if (str2->len){
                str->chr[x]=str2->chr[0];
                x++;
            }
            qbs_free(str2);
            if (stop_program) return str;
            if (x<n){
                evnt(0);//check for new events
                Sleep(10);
                goto waitforinput;
            }
            return str;
        }
    }
