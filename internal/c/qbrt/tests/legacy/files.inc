// Verbatim copy of the legacy GFS layer and file statements from libqb.cpp (commit 545b859), test-only.
// Line ranges: 2554-2629, 7478-7502, 14150-14529, 15038-15299, 15656-15970, 17424-17738, 19356-19358, 24008-24947, 26062-26292.
// ---- lines 2554,2629
struct gfs_file_struct{//info applicable to all files
    int64 id;//a unique ID given to all files (currently only referenced by the FIELD statement to remove old field conditions)
    uint8 open;
    uint8 read;
    uint8 write;
    uint8 lock_read;
    uint8 lock_write;
    int64 pos;//-1=unknown
    uint8 eof_reached;//read last character of file (set/reset by gfs_read only)
    uint8 eof_passed;//attempted to read past eof (set/reset by gfs_read only)
    int32 fileno;//link to fileno index
    uint8 type;//qb access method (1=RANDOM,2=BINARY,3=INPUT,4=OUTPUT)
    int64 record_length;//used by RANDOM
    uint8 *field_buffer;
    qbs **field_strings;//list of qbs pointers linked to this file
    int32 field_strings_n;//number of linked strings
    int64 column;//used by OUTPUT/APPEND to tab correctly (base 0)
    //GFS_C data follows: (unused by custom GFS interfaces)
    fstream *file_handle;
    ofstream *file_handle_o;
    //COM port data follows (*=default)
    uint8 com_port;//0=not a com port
    int32 com_baud_rate;//(bits per second)75,110,150,300*,600,1200,1800,2400,9600,?
    int8 com_parity;//[0]N,[1]E*,[2]O,[3]S,[4]M,[5]PE(none,even*,odd,space,mark,error-checking)
    int8 com_data_bits_per_byte;//5,6,7*,8
    int8 com_stop_bits;//[10]1,[15]1.5,[20]2
    //The default value is 1 for baud rates greater than 110. For
    //baud rates less than or equal to 110, the default value is
    //1.5 when data is 5; otherwise, the value is 2.
    int8 com_bin_asc;//[0]=BIN*,[1]=ASC
    int8 com_asc_lf;//[0]omit*,[1]LF(only valid with ASC)
    //note: rb_x and tb_x are ignored by QBNex (receive and transmit buffer sizes)
    int8 com_rs;//[0]detect*,[1]dont-detect
    int32 com_cd_x;//0*-65535
    int32 com_cs_x;//1000*,0-65535
    int32 com_ds_x;//1000*,0-65535
    int32 com_op_x;
    //                 OP not used:          x omitted:     x specified:
    //                 10 times the CD or    10000 ms       0 - 65,535 milliseconds
    //                 DS timeout value,
    //                 whichever is greater
    
    //SCRN: support follows
    uint8 scrn; //0 = not a file opened as "SCRN:"
};

#ifdef GFS_WINDOWS
    struct gfs_file_win_struct{//info applicable to WINDOWS OS files
        HANDLE file_handle;
    };
    std::vector<gfs_file_win_struct> gfs_file_win;
#endif

int64 gfs_nextid=1;

std::vector<gfs_file_struct> gfs_file;
static std::vector<std::vector<uint8>> gfs_field_buffer_storage;
static std::vector<std::vector<qbs*>> gfs_field_string_storage;

int32 gfs_n=0;
std::vector<int32> gfs_freed;

std::vector<int32> gfs_fileno(1, -1);
int32 gfs_fileno_n=0;

static void sync_gfs_field_strings(int32 index){
    std::vector<qbs*> &buffer=gfs_field_string_storage[index];
    gfs_file[index].field_strings=buffer.empty()?NULL:buffer.data();
    gfs_file[index].field_strings_n=static_cast<int32>(buffer.size());
}

static void sync_gfs_field_buffer(int32 index){
    std::vector<uint8> &buffer=gfs_field_buffer_storage[index];
    gfs_file[index].field_buffer=buffer.empty()?NULL:buffer.data();
}

// ---- lines 7478,7502
void lrset_field(qbs *str);

void sub_lset(qbs *dest,qbs *source){
    if (new_error) return;
    if (source->len>=dest->len){
        if (dest->len) memcpy(dest->chr,source->chr,dest->len);
        goto field_check;
    }
    if (source->len) memcpy(dest->chr,source->chr,source->len);
    memset(dest->chr+source->len,32,dest->len-source->len);
    field_check:
    if (dest->field) lrset_field(dest);
}

void sub_rset(qbs *dest,qbs *source){
    if (new_error) return;
    if (source->len>=dest->len){
        if (dest->len) memcpy(dest->chr,source->chr,dest->len);
        goto field_check;
    }
    if (source->len) memcpy(dest->chr+dest->len-source->len,source->chr,source->len);
    memset(dest->chr,32,dest->len-source->len);
    field_check:
    if (dest->field) lrset_field(dest);
}
// ---- lines 14150,14529
int32 generic_put(int32 i,int32 offset,uint8 *cp,int32 bytes){
    //note: generic_put & generic_get have been made largely redundant by gfs_read & gfs_write
    //      offset is a byte-offset from base 0 (-1=current pos)
    //      generic_put has been kept 32-bit for compatibility
    //      the return value of generic_put is always 0
    //      though errors are handled, generic_put should only be called in error-less situations
    if (new_error) return 0;
    if (gfs_fileno_valid(i)!=1){error(52); return 0;}//Bad file name or number
    i=gfs_fileno[i];//convert fileno to gfs index
    static gfs_file_struct *gfs;
    gfs=&gfs_file[i];
    if (!gfs->write){error(75); return 0;}//Path/file access error
    static int32 e;
    e=gfs_write(i,offset,(uint8*)cp,bytes);
    if (e){
        if (e==-2){error(258); return 0;}//invalid handle
        if (e==-3){error(54); return 0;}//bad file mode
        if (e==-4){error(5); return 0;}//illegal function call
        if (e==-7){error(70); return 0;}//permission denied
        error(75); return 0;//assume[-9]: path/file access error
    }
    return 0;
}

int32 generic_get_bytes_read;
int32 generic_get(int32 i,int32 offset,uint8 *cp,int32 bytes){
    //note: generic_put & generic_get have been made largely redundant by gfs_read & gfs_write
    //      offset is a byte-offset from base 0 (-1=current pos)
    //      generic_get has been kept 32-bit for compatibility
    //      the return value of generic_get is always 0
    //      though errors are handled, generic_get should only be called in error-less situations
    generic_get_bytes_read=0;
    if (new_error) return 0;
    if (gfs_fileno_valid(i)!=1){error(52); return 0;}//Bad file name or number
    i=gfs_fileno[i];//convert fileno to gfs index
    static gfs_file_struct *gfs;
    gfs=&gfs_file[i];
    if (!gfs->read){error(75); return 0;}//Path/file access error
    static int32 e;
    e=gfs_read(i,offset,(uint8*)cp,bytes);
    generic_get_bytes_read=gfs_read_bytes();
    if (e){
        if (e!=-10){//note: on eof, unread buffer area becomes NULL
            if (e==-2){error(258); return 0;}//invalid handle
            if (e==-3){error(54); return 0;}//bad file mode
            if (e==-4){error(5); return 0;}//illegal function call
            if (e==-7){error(70); return 0;}//permission denied
            error(75); return 0;//assume[-9]: path/file access error
        }
    }
    return 0;
}



void sub_open(qbs *name,int32 type,int32 access,int32 sharing,int32 i,int64 record_length,int32 passed){
    if (new_error) return;
    //?[{FOR RANDOM|FOR BINARY|FOR INPUT|FOR OUTPUT|FOR APPEND}]
    //1 2
    //[{ACCESS READ WRITE|ACCESS READ|ACCESS WRITE}]
    //  3
    //[{SHARED|LOCK READ WRITE|LOCK READ|LOCK WRITE}]{AS}[#]?[{LEN =}?]
    //  4                                                   5        6[1]
    static int32 x;
    static int32 g_access,g_restrictions,g_how;
    
    if (type==0) type=1;
    if (passed) if ((record_length==0)||(record_length<-1)){error(5); return;}//Illegal function call
    //note: valid record_length values are allowable but ignored by QB for non-RANDOM modes too!
    
    x=gfs_fileno_valid(i);
    if (x==-2){error(52); return;}//Bad file name or number
    if (x==1){error(55); return;}//File already open
    
    if (type<=2){g_access=3; g_restrictions=0; g_how=3;}
    if (type==3){g_access=1; g_restrictions=0; g_how=0;}
    if (type==4){g_access=2; g_restrictions=0; g_how=2;}
    if (type==5){g_access=2; g_restrictions=0; g_how=1;}
    
    if (access==1) g_access=3;
    if (access==2) g_access=1;
    if (access==3) g_access=2;
    if (access&&(g_how==3)) g_how=1;//undefined access not possible when ACCESS is explicitly specified
    
    if (sharing==1) g_restrictions=0;
    if (sharing==2) g_restrictions=3;
    if (sharing==3) g_restrictions=1;
    if (sharing==4) g_restrictions=2;
    //note: In QB, opening a file already open for OUTPUT/APPEND created the 'file already open' error.
    
    //      However, from a new cmd window (or a SHELLed QB program) it can be opened!
    //      So it is not a true OS restriction/lock, just a block applied internally by QB.
    //      This is currently unsupported by QBNex.
    
    x=gfs_open(name,g_access,g_restrictions,g_how);
    if (x<0){
        if (x==-5){error(53); return;}
        if (x==-6){error(76); return;}
        if (x==-7){error(70); return;}
        if (x==-8){error(68); return;}
        if (x==-11){error(64); return;}
        if (x==-12){error(54); return;}
        error(53); return;//default assumption: 'file not found'
    }
    
    gfs_fileno_use(i,x);
    
    static gfs_file_struct *f; f=&gfs_file[x];
    
    f->type=type; if (type==5) f->type=4;
    
    f->column=1;
    
    if (type==1){//set record length
        f->record_length=128;
        if (passed) if (record_length!=-1) f->record_length=record_length;
        gfs_field_buffer_storage[x].assign(f->record_length,0);
        sync_gfs_field_buffer(x);
    }
    
    if (type==5){//seek eof
        static int64 x64;
        x64=gfs_lof(x);
        if (x64>0) gfs_setpos(x,x64);//not an error and not null length
    }
    
    if (type==3){//check if eof character, CHR$(26), is the first byte and set EOF accordingly
        static int64 x64;
        x64=gfs_lof(x);
        if (x64){
            //read first byte 
            static uint8 c;
            static int32 e;
            if (e=gfs_read(x,-1,&c,1)){
                //if (e==-10) return -1;
                //if (e==-2){error(258); return -2;}//invalid handle
                //if (e==-3){error(54); return -2;}//bad file mode
                //if (e==-4){error(5); return -2;}//illegal function call
                if (e==-7){error(70); return;}//permission denied
                error(75); return;//assume[-9]: path/file access error
            }
            if (c==26){
                gfs_file[x].eof_passed=1;//set EOF flag
            }
            gfs_setpos(x,0);
        }
    }//type==3
    
}

void sub_open_gwbasic(qbs *typestr,int32 i,qbs *name,int64 record_length,int32 passed){
    if (new_error) return;
    static int32 a,type;
    if (!typestr->len){error(54); return;}//bad file mode
    a=typestr->chr[0]&223;
    type=0;
    if (a==82) type=1;//R
    if (a==66) type=2;//B
    if (a==73) type=3;//I
    if (a==79) type=4;//O
    if (a==65) type=5;//A
    if (!type){error(54); return;}//bad file mode
    if (passed){
        sub_open(name,type,NULL,NULL,i,record_length,1);
        }else{
        sub_open(name,type,NULL,NULL,i,NULL,0);
    }
}

void sub_close(int32 i2,int32 passed){
    if (new_error) return;
    int32 i,x;//<--RECURSIVE function - do not make this static
    
    if (passed){
        
        if (i2<0){//special handle
            //determine which close procedure to call
            x=-(i2+1);
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh) return;
            if (sh->type==1){//stream
                static stream_struct *st; st=(stream_struct*)sh->index;
                if (st->type==1){//connection
                    connection_close(x);
                }//connection
            }//stream
            
            if (sh->type==2){//host listener
                connection_close(x);
            }//host listener
            
            return;
        }//special handle
        
        
        if (gfs_fileno_valid(i2)==1) gfs_close(gfs_fileno[i2]);
        return;
        
    }//passed
    
    //special handles
    
    
    
    for (i=1;i<=special_handles->indexes;i++){
        if (vwatch>0 && vwatch==i) {
            //keep the debug connection open for $DEBUG mode
        } else {
            sub_close(-i-1,1);
        }
    }
    
    
    for (i=1;i<=gfs_fileno_n;i++){
        if (gfs_fileno_valid(i)==1) gfs_close(gfs_fileno[i]);
    }
    
}//close

int32 file_input_chr(int32 i){
    //returns the ASCII value of the character (0-255)
    //returns -1 if eof reached (error to be externally handled)
    //returns -2 for other errors (internally handled), the calling function should abort
    
    static uint8 c;
    static int32 e;
    if (e=gfs_read(i,-1,&c,1)){
        if (e==-10) return -1;
        if (e==-2){error(258); return -2;}//invalid handle
        if (e==-3){error(54); return -2;}//bad file mode
        if (e==-4){error(5); return -2;}//illegal function call
        if (e==-7){error(70); return -2;}//permission denied
        error(75); return -2;//assume[-9]: path/file access error
    }
    if (c==26){//eof character (go back 1 byte so subsequent reads will re-encounter the eof character)
        gfs_setpos(i,gfs_getpos(i)-1);
        gfs_file[i].eof_passed=1;//also set EOF flag
        return -1;
    }
    return c;
    
}

void file_input_skip1310(int32 i,int32 c){
    //assumes a character of value 13 or 10 has just been read (passed)
    //peeks next character and skips it too if it is a corresponding 13 or 10 pair
    static int32 nextc;
    nextc=file_input_chr(i);
    if (nextc==-2) return;
    if (nextc==-1) return;
    if (((c==10)&&(nextc!=13))||((c==13)&&(nextc!=10))){
        gfs_setpos(i,gfs_getpos(i)-1);//go back 1 character
        }else{
        //check next character for EOF CHR$(26)
        nextc=file_input_chr(i);
        if (nextc==-2) return;
        if (nextc==-1) return;
        gfs_setpos(i,gfs_getpos(i)-1);//go back 1 character
    }
}

void file_input_nextitem(int32 i,int32 lastc){
    if (i<0) return;
    //this may require reversing a bit too!
    int32 c,nextc;
    c=lastc;
    nextchr:
    if (c==-1) return;
    if (c==32){
        nextc=file_input_chr(i); if (nextc==-2) return;
        if (nextc==-1) return;
        if ( (nextc!=32)&&(nextc!=44)&&(nextc!=10)&&(nextc!=13) ){
            gfs_setpos(i,gfs_getpos(i)-1);
            return;
            }else{
            c=nextc;
            goto nextchr;
        }
    }
    if (c==44) return;//,
    if ((c==10)||(c==13)){//lf cr
        file_input_skip1310(i,c);
        return;
    }
    c=file_input_chr(i); if (c==-2) return;
    goto nextchr;
}

uint8 sub_file_print_spaces[32];
void sub_file_print(int32 i,qbs *str,int32 extraspace,int32 tab,int32 newline){
    if (new_error) return;
    static int32 x,x2,x3,x4;
    static int32 e;
    
    //tcp/ip?
    //note: spacing considerations such as 'extraspace' & 'tab' are ignored
    if (i<0){
        
        return;
    }
    if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
    i=gfs_fileno[i];//convert fileno to gfs index
    static gfs_file_struct *gfs;
    gfs=&gfs_file[i];
    
    if (gfs->scrn==1) {
        qbs_print (str, newline);
        return;
    };
    
    if (gfs->type!=4){error(54); return;}//Bad file mode
    if (!gfs->write){error(75); return;}//Path/file access error
    
    e=gfs_write(i,-1,str->chr,str->len);
    if (e){
        if (e==-2){error(258); return;}//invalid handle
        if (e==-3){error(54); return;}//bad file mode
        if (e==-4){error(5); return;}//illegal function call
        if (e==-7){error(70); return;}//permission denied
        error(75); return;//assume[-9]: path/file access error
    }
    
    //move column if carriage return found in content
    static int32 stri,strl;
    static uint8 c;
    strl=str->len;
    for (stri=0;stri<strl;stri++){
        c=str->chr[stri];
        if ((c!=13)&&(c!=10)){
            gfs->column++;
            }else{
            if (c==13) gfs->column=1;
        }
    }
    
    //add extra spaces as needed
    static int32 nspaces;
    static int16 cr_lf=13+10*256; 
    nspaces=0;
    if (extraspace){
        nspaces++;
        gfs->column++;
    }
    if (tab){
        //a space MUST be added
        nspaces++;
        gfs->column++;
        x=(gfs->column-1)%14;
        if (x!=0){
            x=14-x;
            nspaces+=x;
            gfs->column+=x;
        }
    }
    if (nspaces){
        
        e=gfs_write(i,-1,sub_file_print_spaces,nspaces);
        if (e){
            if (e==-2){error(258); return;}//invalid handle
            if (e==-3){error(54); return;}//bad file mode
            if (e==-4){error(5); return;}//illegal function call
            if (e==-7){error(70); return;}//permission denied
            error(75); return;//assume[-9]: path/file access error
        }
        
    }
    if (newline){
        
        e=gfs_write(i,-1,(uint8*)&cr_lf,2);
        if (e){
            if (e==-2){error(258); return;}//invalid handle
            if (e==-3){error(54); return;}//bad file mode
            if (e==-4){error(5); return;}//illegal function call
            if (e==-7){error(70); return;}//permission denied
            error(75); return;//assume[-9]: path/file access error
        }
        
        gfs->column=1;
    }
}

// ---- lines 15038,15299
int32 n_inputnumberfromfile(int32 fileno){
    //return values:
    //0=success
    //1=overflow
    //2=eof
    //3=failed (no further errors)
    
    //data
    static int32 i,i2;
    static int32 step,c;
    static int32 exponent_digits;
    static uint8 negate_exponent;
    static int64 exponent_value;
    static int32 return_value;
    
    //tcp/ip specific data
    static qbs *str,*character;
    int32 nextc,x,x2,x3,x4;
    int32 i1;
    int32 inspeechmarks;
    static uint8 *ucbuf;
    static uint32 ucbufsiz;
    static int32 info;
    
    if (fileno>=0){
        if (gfs_fileno_valid(fileno)!=1){error(52); return 3;}//Bad file name or number
        fileno=gfs_fileno[fileno];//convert fileno to gfs index
        static gfs_file_struct *gfs;
        gfs=&gfs_file[fileno];
        if (gfs->type!=3){error(54); return 3;}//Bad file mode
        if (!gfs->read){error(75); return 3;}//Path/file access error
    }
    
    return_value=1;//overflow (default)
    step=0;
    negate_exponent=0;
    exponent_value=0;
    exponent_digits=0;
    
    //prepare universal number representation
    n_digits=0; n_exp=0; n_neg=0; n_hex=0;
    
    //skip any leading spaces
    do{
        c=file_input_chr(fileno); if (c==-2) return 3;
        if (c==-1){return_value=2; goto error;}//input past end of file
    }while(c==32);
    
    // hex/oct/bin
    if (c == 38) {  // "&"
      c = file_input_chr(fileno);
      if (c == -2) return 3;
      if (c == -1) goto gotnumber;
      if ((c == 72) || (c == 104)) {  // "H" or "h"
      nexthexchr:
        c = file_input_chr(fileno);
        if (c == -2) return 3;
        if (((c >= 48) && (c <= 57)) || ((c >= 65) && (c <= 70)) ||
            ((c >= 97) && (c <= 102))) {    // 0-9 or A-F or a-f
          if (n_digits == 256) goto error;  // overflow
          n_digit[n_digits] = c;
          n_digits++;
          n_hex = 1;
          goto nexthexchr;
        }
        goto gotnumber;
      }
      if ((c == 79) || (c == 111)) {  // "O" or "o"
      nexthexchr2:
        c = file_input_chr(fileno);
        if (c == -2) return 3;
        if ((c >= 48) && (c <= 55)) {       // 0-7
          if (n_digits == 256) goto error;  // overflow
          n_digit[n_digits] = c;
          n_digits++;
          n_hex = 2;
          goto nexthexchr2;
        }
        goto gotnumber;
      }
      if ((c == 66) || (c == 98)) {  // "B" or "b"
      nexthexchr3:
        c = file_input_chr(fileno);
        if (c == -2) return 3;
        if ((c >= 48) && (c <= 49)) {       // 0-1
          if (n_digits == 256) goto error;  // overflow
          n_digit[n_digits] = c;
          n_digits++;
          n_hex = 3;
          goto nexthexchr3;
        }
        goto gotnumber;
      }
      goto gotnumber;
    }  // "&"
    
    readnextchr:
    if (c==-1) goto gotnumber;
    
    if (c==45){//-
        if (step==0){n_neg=1; step=1; goto nextchr;}
        if (step==3){negate_exponent=1; step=4; goto nextchr;}
        goto gotnumber;
    }
    
    if (c==43){//+
        if (step==0){step=1; goto nextchr;}
        if (step==3){step=4; goto nextchr;}
        goto gotnumber;
    }
    
    if ((c>=48)&&(c<=57)){//0-9
        
        if (step<=1){//before decimal point
            step=1;
            if (n_digits||(c>48)){
                if (n_digits) n_exp++;
                if (n_digits==256) goto error;//overflow
                n_digit[n_digits]=c;
                n_digits++;
            }
        }
        
        if (step==2){//after decimal point
            if ((n_digits==0)&&(c==48)) n_exp--;
            if ((n_digits)||(c>48)){
                if (n_digits==256) goto error;//overflow
                n_digit[n_digits]=c;
                n_digits++;
            }
        }
        
        if (step>=3){//exponent
            step=4;
            if ((exponent_digits)||(c>48)){
                if (exponent_digits==18) goto error;//overflow
                exponent_value*=10;
                exponent_value=exponent_value+(c-48);
                exponent_digits++;
            }
        }
        
        goto nextchr;
    }
    
    if (c==46){//.
        if (step>1) goto gotnumber;
        if (n_digits==0) n_exp=-1;
        step=2; goto nextchr;
    }
    
    if ((c==68)||(c==69)||(c==100)||(c==101)){//D,E,d,e
        if (step>2) goto gotnumber;
        step=3; goto nextchr;
    }
    
    goto gotnumber;//invalid character
    nextchr:
    c=file_input_chr(fileno); if (c==-2) return 3;
    goto readnextchr;
    
    gotnumber:;
    if (negate_exponent) n_exp-=exponent_value; else n_exp+=exponent_value;//complete exponent
    if (n_digits==0) {n_exp=0; n_neg=0;}//clarify number
    file_input_nextitem(fileno,c);
    return 0;//success
    
    error:
    file_input_nextitem(fileno,c);
    if (fileno<0){
        
    }
    return return_value;
}


void revert_input_check(){
}

void sub_file_input_string(int32 fileno,qbs *deststr){
    if (new_error) return;
    static qbs *str,*character;
    int32 c,nextc,x,x2,x3,x4;
    int32 i,i1;
    int32 inspeechmarks;
    static uint8 *ucbuf;
    static uint32 ucbufsiz;
    static int32 info;
    
    //tcp/ip?
    //note: spacing considerations are ignored
    if (fileno<0){
        return;
    }
    
    if (gfs_fileno_valid(fileno)!=1){error(52); return;}//Bad file name or number
    fileno=gfs_fileno[fileno];//convert fileno to gfs index
    
    static gfs_file_struct *gfs;
    gfs=&gfs_file[fileno];
    if (gfs->type!=3){error(54); return;}//Bad file mode
    if (!gfs->read){error(75); return;}//Path/file access error
    
    str=qbs_new(0,0);
    //skip whitespace (spaces & tabs)
    do{
        c=file_input_chr(fileno); if (c==-2) return;
        if (c==-1){
            qbs_set(deststr,str);
            qbs_free(str);
            error(62);//input past end of file
            return;
        }
    }while((c==32)||(c==9));
    //quoted string?
    inspeechmarks=0;
    if (c==34){//"
        inspeechmarks=1;
        c=file_input_chr(fileno);
    }
    //read string
    character=qbs_new(1,0);
    nextchr:
    if (c==-2) return;
    if (c==-1) goto gotstr;
    if (inspeechmarks){
        if (c==34) goto gotstr;//"
        }else{
        if (c==44) goto gotstr;//,
        if (c==10) goto gotstr;
        if (c==13) goto gotstr;
    }
    character->chr[0]=c; qbs_set(str,qbs_add(str,character));
    c=file_input_chr(fileno);
    goto nextchr;
    gotstr:
    //cull trailing whitespace
    if (!inspeechmarks){
        cullstr:
        if (str->len){
            if ((str->chr[str->len-1]==32)||(str->chr[str->len-1]==9)){str->len--; goto cullstr;}
        }
    }
    nextstr:
    //scan until next item (or eof) reached
    if (c==-2) return;
    if (c==-1) goto returnstr;
    if (c==44) goto returnstr;
    if ((c==10)||(c==13)){//lf cr
        file_input_skip1310(fileno,c);
        goto returnstr;
    }
    c=file_input_chr(fileno);
    goto nextstr;
    //return string
    returnstr:
    qbs_set(deststr,str);
    qbs_free(str);
    qbs_free(character);
    return;
}

// ---- lines 15656,15970
void sub_get(int32 i,int64 offset,void *element,int32 passed){
    if (new_error) return;
    static byte_element_struct *ele;
    static int32 x,x2;
    
    if (i<0){//special handle?
        x=-(i+1);
        static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); return;}
        if (sh->type==1){//stream
            static stream_struct *st; st=(stream_struct*)sh->index;
            stream_update(st);
            ele=(byte_element_struct*)element;
            if (st->in_size<ele->length){st->eof=1; return;}
            st->eof=0;
            memcpy((void*)(ele->offset),st->in,ele->length);
            x2=st->in_size-ele->length; if (x2) memmove(st->in,st->in+ele->length,x2);
            st->in_size-=ele->length;
            return;
        }//stream
        error(52); return;
    }//special handle
    
    if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
    i=gfs_fileno[i];//convert fileno to gfs index
    static gfs_file_struct *gfs;
    gfs=&gfs_file[i];
    if (gfs->type>2){error(54); return;}//Bad file mode
    if (!gfs->read){error(75); return;}//Path/file access error
    
    ele=(byte_element_struct*)element;
    
    if (gfs->type==1){//RANDOM
        if (ele->length>gfs->record_length){error(59); return;}//Bad record length
        if (passed){
            offset--;
            if (offset<0){error(63); return;}//Bad record number
            offset*=gfs->record_length;
            }else{
            offset=-1;
        }
        }else{//BINARY
        if (passed){
            offset--;
            if (offset<0){error(63); return;}//Bad record number
        }else{offset=-1;}
    }
    
    static int32 e;
    
    e=gfs_read(i,offset,(uint8*)ele->offset,ele->length);
    if (e){
        if (e!=-10){//note: on eof, unread buffer area becomes NULL
            if (e==-2){error(258); return;}//invalid handle
            if (e==-3){error(54); return;}//bad file mode
            if (e==-4){error(5); return;}//illegal function call
            if (e==-7){error(70); return;}//permission denied
            error(75); return;//assume[-9]: path/file access error
        }
    }
    
    //seek to beginning of next field
    if (gfs->type==1){
        if (e!=-10){//note: seek index not advanced if record did not exist
            if (ele->length<gfs->record_length){
                if (offset!=-1){
                    e=gfs_setpos(i,offset+gfs->record_length);
                    }else{
                    e=gfs_setpos(i,gfs_getpos(i)-ele->length+gfs->record_length);
                }
                if (e){error(54); return;}//assume[-3]: bad file mode
            }
        }//e!=-10
    }
    
}//get

void sub_get2(int32 i,int64 offset,qbs *str,int32 passed){
    if (new_error) return;
    static int32 x,x2,x3,x4;
    
    
    if (i<0){//special handle?
        if (str->fixed){//following method is only for variable length strings
            static byte_element_struct tbyte_element_struct;
            sub_get(i,offset,byte_element((uint64)str->chr,str->len,&tbyte_element_struct),passed);
            return;
        }
        x=-(i+1);
        static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); return;}
        if (sh->type==1){//stream
            static stream_struct *st; st=(stream_struct*)sh->index;
            stream_update(st);
            static qbs* tqbs;
            tqbs=qbs_new(st->in_size,1);
            if (st->in_size) memcpy(tqbs->chr,st->in,st->in_size);
            st->in_size=0;
            st->eof=0;
            qbs_set(str,tqbs);
            return;
        }//stream
        error(52); return;
    }//special handle
    
    
    if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
    i=gfs_fileno[i];//convert fileno to gfs index
    static gfs_file_struct *gfs;
    gfs=&gfs_file[i];
    if (gfs->type>2){error(54); return;}//Bad file mode
    if (!gfs->read){error(75); return;}//Path/file access error
    
    if (gfs->type==2){//BINARY (use normal sub_get)
        static byte_element_struct tbyte_element_struct;
        sub_get(gfs->fileno,offset,byte_element((uint64)str->chr,str->len,&tbyte_element_struct),passed);
        return;
    }
    
    if (gfs->record_length<2){error(59); return;}//Bad record length
    
    if (passed){
        offset--;
        if (offset<0){error(63); return;}//Bad record number
        offset*=gfs->record_length;
        }else{
        offset=-1;
    }
    
    static int32 e;
    
    static std::vector<uint8> data_storage;
    static uint64 l,bytes;
    data_storage.resize(gfs->record_length);
    uint8 *data=data_storage.data();
    e=gfs_read(i,offset,data,gfs->record_length);//read the whole record (including header & data)
    if (e){
        if (e!=-10){//note: on eof, unread buffer area becomes NULL
            if (e==-2){error(258); return;}//invalid handle
            if (e==-3){error(54); return;}//bad file mode
            if (e==-4){error(5); return;}//illegal function call
            if (e==-7){error(70); return;}//permission denied
            error(75); return;//assume[-9]: path/file access error
        }
    }
    
    bytes=gfs_read_bytes();//note: any unread part of the buffer is set to NULL (by gfs_read) and is treated as valid record data
    if (!bytes){qbs_set(str,qbs_new(0,1)); return;}//as in QB when 0 bytes read, NULL string returned and (as no bytes read) no seek advancement
    
    //seek to beginning of next field
    //note: advancement occurs even if e==-10 (eof reached)
    if (bytes<gfs->record_length){
        if (offset!=-1){
            e=gfs_setpos(i,offset+gfs->record_length);
            }else{
            e=gfs_setpos(i,gfs_getpos(i)-bytes+gfs->record_length);
        }
        if (e){error(54); return;}//assume[-3]: bad file mode
    }
    
    x=2;//offset where string data will be read from
    l=((uint16*)data)[0];
    if (l&32768){
        if (gfs->record_length<8){//record length is too small to read the length!
            //restore seek to original location
            if (offset!=-1){
                e=gfs_setpos(i,offset);
                }else{
                e=gfs_setpos(i,gfs_getpos(i)-gfs->record_length);
            }
            error(59); return;//Bad record length
        }
        x=8;
        l=(l&0x7FFF)+( ( (((uint64*)data)[0]) >> 16) << 15 );
    }
    
    //can record_length cannot fit the required string data?
    if ((gfs->record_length-x2)<l){
        //restore seek to original location
        if (offset!=-1){
            e=gfs_setpos(i,offset);
            }else{
            e=gfs_setpos(i,gfs_getpos(i)-gfs->record_length);
        }
        error(59); return;//Bad record length
    }
    
    qbs_set(str,qbs_new_txt_len((char*)(data+x),l));
}

void sub_put(int32 i,int64 offset,void *element,int32 passed){
    if (new_error) return;
    static byte_element_struct *ele;
    static int32 x,x2;
    
    if (i<0){//special handle?
        x=-(i+1);
        static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); return;}
        if (sh->type==1){//stream
            static stream_struct *st; st=(stream_struct*)sh->index;
            ele=(byte_element_struct*)element;
            stream_out(st,(void*)ele->offset,ele->length);
            return;
        }//stream
        error(52); return;
    }//special handle
    
    if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
    i=gfs_fileno[i];//convert fileno to gfs index
    static gfs_file_struct *gfs;
    gfs=&gfs_file[i];
    if (gfs->type>2){error(54); return;}//Bad file mode
    if (!gfs->write){error(75); return;}//Path/file access error
    
    ele=(byte_element_struct*)element;
    
    if (gfs->type==1){//RANDOM
        if (ele->length>gfs->record_length){error(59); return;}//Bad record length
        if (passed){
            offset--;
            if (offset<0){error(63); return;}//Bad record number
            offset*=gfs->record_length;
            }else{
            offset=-1;
        }
        }else{//BINARY
        if (passed){
            offset--;
            if (offset<0){error(63); return;}//Bad record number
        }else{offset=-1;}
    }
    
    static int32 e;
    
    e=gfs_write(i,offset,(uint8*)ele->offset,ele->length);
    if (e){
        if (e==-2){error(258); return;}//invalid handle
        if (e==-3){error(54); return;}//bad file mode
        if (e==-4){error(5); return;}//illegal function call
        if (e==-7){error(70); return;}//permission denied
        error(75); return;//assume[-9]: path/file access error
    }
    
    //seek to beginning of next field
    if (gfs->type==1){
        if (ele->length<gfs->record_length){
            if (offset!=-1){
                e=gfs_setpos(i,offset+gfs->record_length);
                }else{
                e=gfs_setpos(i,gfs_getpos(i)-ele->length+gfs->record_length);
            }
            if (e) error(54); return;//assume[-3]: bad file mode
        }
    }
    
}

//put2 adds a 2-4 byte length descriptor to the data
//(used to PUT variable length strings in RANDOM mode)
void sub_put2(int32 i,int64 offset,void *element,int32 passed){
    if (new_error) return;
    static byte_element_struct *ele;
    static int32 x;
    static std::vector<uint8> data_storage;
    
    if (i<0){//special handle?
        sub_put(i,offset,element,passed);//(use standard put call)
        return;
    }
    
    if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
    i=gfs_fileno[i];//convert fileno to gfs index
    static gfs_file_struct *gfs;
    gfs=&gfs_file[i];
    if (gfs->type>2){error(54); return;}//Bad file mode
    if (!gfs->write){error(75); return;}//Path/file access error
    
    if (gfs->type==2){//BINARY (use normal sub_put)
        sub_put(gfs->fileno,offset,element,passed);
        return;
    }
    
    //RANDOM
    static uint64 l;
    static int64 lmask;
    lmask=-1;
    lmask>>=16;
    ele=(byte_element_struct*)element;
    l=ele->length;//note: ele->length is currently 32-bit, but sub_put2 is 64-bit compliant
    //{15}{1}[{48}]
        if (l>32767){
            data_storage.resize(static_cast<size_t>(l)+8);
            uint8 *data=data_storage.data();
            memcpy(&data[8],(void*)(ele->offset),l);
            ((uint64*)data)[0]=0;
            ((uint16*)data)[0]=(l&32767)+32768;
            l=((l>>15)&lmask);
            ((uint64*)(data+2))[0]|=l;
            ele->length+=8;
            }else{
            data_storage.resize(static_cast<size_t>(l)+2);
            uint8 *data=data_storage.data();
            memcpy(&data[2],(void*)(ele->offset),l);
            ((uint16*)data)[0]=l;
            ele->length+=2;
        }
        uint8 *data=data_storage.data();
        ele->offset=(uint64)&data[0];
        sub_put(gfs->fileno,offset,element,passed);
        
    }//put2
    
    
    
    
    
    
// ---- lines 17424,17738
    int64 func_lof(int32 i){
        static int64 size;
        
        if (i<0){//special handle?
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,-(i+1)); if (!sh){error(52); return 0;}
            if (sh->type==1){//stream
                static stream_struct *st; st=(stream_struct*)sh->index;
                stream_update(st);
                return st->in_size;
            }//stream
            error(52); return 0;
        }//special handle
        
        if (gfs_fileno_valid(i)!=1){error(52); return 0;}//Bad file name or number
        i=gfs_fileno[i];//convert fileno to gfs index
        size=gfs_lof(i);
        if (size<0){
            if (size==-2){error(258); return 0;}//invalid handle
            if (size==-3){error(54); return 0;}//bad file mode
            if (size==-4){error(5); return 0;}//illegal function call
            error(75); return 0;//assume[-9]: path/file access error
        }
        return size;
    }
    
    int32 func_eof(int32 i){
        static int32 pos,lof,x;
        
        if (i<0){//special handle?
            x=-(i+1);
            static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); return 0;}
            if (sh->type==1){//stream
                static stream_struct *st; st=(stream_struct*)sh->index;
                if (st->eof) return -1;
                return 0;
            }//stream
            error(52); return 0;
        }//special handle
        
        if (gfs_fileno_valid(i)!=1){error(52); return 0;}//Bad file name or number
        i=gfs_fileno[i];//convert fileno to gfs index
        static gfs_file_struct *gfs;
        gfs=&gfs_file[i];
        if (gfs->scrn){error(5); return 0;}
        if (gfs->type!=3){  //uint8 type;//qb access method (1=RANDOM,2=BINARY,3=INPUT,4=OUTPUT)
            if (gfs_eof_passed(i)==1) return -1;
            }else{
            if (gfs_eof_reached(i)==1) return -1;
            if (gfs_eof_passed(i)==1) return -1;
        }
        return 0;
        
    }
    
    void sub_seek(int32 i,int64 pos){
        if (new_error) return;
        if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
        i=gfs_fileno[i];//convert fileno to gfs index
        static gfs_file_struct *gfs;
        gfs=&gfs_file[i];
        if (gfs->type==1){//RANDOM
            pos--;
            if (pos<0){error(63); return;}//Bad record number
            pos*=gfs->record_length;
            pos++;
        }
        pos--;
        if (pos<0){error(63); return;}//Bad record number
        int32 e;
        e=gfs_setpos(i,pos);
        if (e<0){
            if (e==-2){error(258); return;}//invalid handle
            if (e==-3){error(54); return;}//bad file mode
            if (e==-4){error(5); return;}//illegal function call
            error(75); return;//assume[-9]: path/file access error
        }
    }
    
    int64 func_seek(int32 i){
        if (gfs_fileno_valid(i)!=1){error(52); return 0;}//Bad file name or number
        i=gfs_fileno[i];//convert fileno to gfs index
        static gfs_file_struct *gfs;
        gfs=&gfs_file[i];
        if (gfs->scrn) return 0;
        if (gfs->type==1){//RANDOM
            return gfs_getpos(i)/gfs->record_length+1;
        }
        return gfs_getpos(i)+1;
    }
    
    int64 func_loc(int32 i){
        if (gfs_fileno_valid(i)!=1){error(52); return 0;}//Bad file name or number
        i=gfs_fileno[i];//convert fileno to gfs index
        static gfs_file_struct *gfs;
        gfs=&gfs_file[i];
        
        if (gfs->scrn){error(5); return 0;}
        if (gfs->com_port){
            #ifdef QBNex_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                static COMSTAT c;
                ZeroMemory(&c,sizeof(COMSTAT));
                static DWORD ignore;
                if (!ClearCommError(f_w->file_handle,&ignore,&c)) return 0;
                return c.cbInQue;//bytes in COM input buffer
            #endif
        }
        
        if (gfs->type==1){//RANDOM
            return gfs_getpos(i)/gfs->record_length;
        }
        if (gfs->type==2){//BINARY
            return gfs_getpos(i);
        }
        //APPEND/OUTPUT/INPUT
        return gfs_getpos(i)/128;
    }
    
    qbs *func_input(int32 n,int32 i,int32 passed){
        if (new_error) return qbs_new(0,1);
        static qbs *str,*str2;
        static int32 x,c;
        if (n<0) str=qbs_new(0,1); else str=qbs_new(n,1);
        if (passed){
            
            if (gfs_fileno_valid(i)!=1){error(52); return str;}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            static gfs_file_struct *gfs;
            gfs=&gfs_file[i];
            if ((gfs->type<2)||(gfs->type>3)){error(62); return str;}//Input past end of file
            //note: RANDOM should be supported
            //note: Unusually, QB returns "Input past end of file" instead of "Bad file mode"
            if (!gfs->read){error(75); return str;}//Path/file access error
            
            if (n<0){error(52); return str;}//Bad file name or number
            if (n==0) return str;
            
            //INPUT -> Input past end of file at EOF or CHR$(26)
            //         unlike BINARY, partial strings cannot be returned
            //         (use input_file_chr and modify it to support CHR$(26)
            if (gfs->type==3){
                x=0;
                do{
                    c=file_input_chr(i);
                    if (c==-1){error(62); return str;}//Input past end of file
                    if (c==-2){error(75); return str;}//path/file access error
                    str->chr[x]=c;
                    
                    if (gfs_file[i].eof_passed!=1) { //If we haven't declared the End of the File, check to see if the next byte is an EOF byte
                        c=file_input_chr(i); //read the next byte
                        if (gfs_file[i].eof_passed!=1) {gfs_setpos(i,gfs_getpos(i)-1);} //and if it's not EOF, move our position back to where it should be
                    }
                    
                    x++;
                }while(x<n);
                return str;
            }
            
            //BINARY -> If past EOF, returns a NULL length string!
            //          or as much of the data that was valid as possible
            //          Seek POS is only advanced as far as was read!
            //          Binary can read past CHR$(26)
            //          (simply call C's read statement and manage via .eof
            if (gfs->type==2){
                static int32 e;
                e=gfs_read(i,-1,str->chr,n);
                if (e){
                    if (e!=-10){//note: on eof, unread buffer area becomes NULL
                        str->len=0;
                        if (e==-2){error(258); return str;}//invalid handle
                        if (e==-3){error(54); return str;}//bad file mode
                        if (e==-4){error(5); return str;}//illegal function call
                        if (e==-7){error(70); return str;}//permission denied
                        error(75); return str;//assume[-9]: path/file access error
                    }
                }
                str->len=gfs_read_bytes();//adjust if not enough data was available
                return str;
            }
            
            //RANDOM (todo)
            
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
    
    void file_line_input_string_character(int32 filehandle, qbs *deststr) {
        static qbs *str,*character;
        int32 c,nextc;
        int32 inspeechmarks;
        
        str=qbs_new(0,0);
        c=file_input_chr(filehandle); if (c==-2) return;
        if (c==-1){
            qbs_set(deststr,str);
            qbs_free(str);
            error(62);//input past end of file
            return;
        }
        character=qbs_new(1,0);
        nextchr:
        if (c==-1) goto gotstr;
        if (c==10) goto gotstr;
        if (c==13) goto gotstr;
        character->chr[0]=c; qbs_set(str,qbs_add(str,character));
        c=file_input_chr(filehandle); if (c==-2) return;
        goto nextchr;
        gotstr:
        nextstr:
        //scan until next item (or eof) reached
        if (c==-1) goto returnstr;
        if ((c==10)||(c==13)){//lf cr
            file_input_skip1310(filehandle,c);
            goto returnstr;
        }
        c=file_input_chr(filehandle); if (c==-2) return;
        goto nextstr;
        //return string
        returnstr:
        qbs_set(deststr,str);
        qbs_free(str);
        qbs_free(character);
        return;
    }
    
    
    void file_line_input_string_binary(int32 fileno, qbs *deststr) {
        int32 filebuf_size = 512;
        int32 filehandle;
        qbs *eol;
        
        filehandle=gfs_fileno[fileno];//convert fileno to gfs index
        eol = qbs_new_txt_len("\n", 1);
        
        int64 start_byte = func_seek(fileno);
        int64 filelength = func_lof(fileno);
        if (start_byte > filelength) {
            error(62);//input past end of file
            return;
        }
        qbs *buffer = qbs_new(filebuf_size, 0);
        qbs_set(deststr, qbs_new_txt_len("", 0));
        do {
            if (start_byte + filebuf_size > filelength) filebuf_size = filelength - start_byte + 1;
            qbs_set(buffer,func_space(qbr(filebuf_size))); 
            
            sub_get2(fileno, start_byte, buffer, 1);
            int32 eol_pos = func_instr(0, buffer, eol, 0);
            if (eol_pos == 0) {
                if ((start_byte + filebuf_size)>=filelength) {
                    qbs_set(deststr, buffer);
                    gfs_setpos(filehandle,filelength); //set the position right before the EOF marker
                    gfs_file[filehandle].eof_passed=1;//also set EOF flag;
                    qbs_free(buffer);
                    return;
                }
                filebuf_size += 512;
            }
            else {
                qbs_set(deststr, qbs_add(deststr, qbs_left(buffer, eol_pos - 1)));
                break;
            }
        } while (!func_eof(fileno));
        qbs_free(buffer);
        if (start_byte + deststr->len + 2 >= filelength) { //if we've read to the end of the line
            gfs_setpos(filehandle,filelength); //set the position right before the EOF marker
            gfs_file[filehandle].eof_passed=1;//also set EOF flag;
            if (deststr->chr[deststr->len - 1] == '\r') qbs_set(deststr, qbs_left(deststr, deststr->len-1));
            return;
        }
        gfs_setpos(filehandle,start_byte + deststr->len); //set the position at the end of the text
        if (deststr->chr[deststr->len - 1] == '\r') qbs_set(deststr, qbs_left(deststr, deststr->len-1));
    }
    
    
    void sub_file_line_input_string(int32 fileno, qbs *deststr){
        int32 filehandle;
        if (new_error) return;
        if (gfs_fileno_valid(fileno)!=1){error(52); return;}//Bad file name or number
        filehandle=gfs_fileno[fileno];//convert fileno to gfs index
        static gfs_file_struct *gfs;
        gfs=&gfs_file[filehandle];
        if (!gfs->read){error(75); return;}//Path/file access error
        
        if (gfs->type == 2) {
            file_line_input_string_binary(fileno, deststr);
        }
        else if (gfs->type == 3) {
            file_line_input_string_character(filehandle, deststr);
            } else {
            error(54); //Bad file mode
        }
        return;
    }
    
// ---- lines 19356,19358
        int32 func_freefile(){
            return gfs_fileno_freefile();
        }
// ---- lines 24008,24947
        int32 gfs_new(){
            static int32 i;
            if (!gfs_freed.empty()){
                i=gfs_freed.back();
                gfs_freed.pop_back();
                }else{
                i=gfs_n;
                gfs_n++;
                gfs_file.resize(gfs_n);
                gfs_field_buffer_storage.resize(gfs_n);
                gfs_field_string_storage.resize(gfs_n);
                #ifdef GFS_WINDOWS
                    gfs_file_win.resize(gfs_n);
                #endif
            }
            memset(&gfs_file[i],0,sizeof(gfs_file_struct));
            gfs_field_buffer_storage[i].clear();
            sync_gfs_field_buffer(i);
            gfs_field_string_storage[i].clear();
            sync_gfs_field_strings(i);
            #ifdef GFS_WINDOWS
                ZeroMemory(&gfs_file_win[i],sizeof(gfs_file_win_struct));
            #endif
            gfs_file[i].id=gfs_nextid++;
            return i;
        }
        
        int32 gfs_validhandle(int32 i){
            if ((i<0)||(i>=gfs_n)) return 0;
            if (gfs_file[i].scrn) return 1;
            if (gfs_file[i].open) return 1;
            return 0;
        }
        
        int32 gfs_fileno_valid(int32 f){
            //returns: -2   invalid handle
            //          1   in use
            //          0   unused
            
            if (f<=0) return -2;
            if (f<=gfs_fileno_n){
                if (gfs_fileno[f]==-1) return 0; else return 1;
            }
            gfs_fileno.resize(f+1,-1);
            gfs_fileno_n=f;
            return 0;
        }
        
        int32 gfs_fileno_freefile(){//like FREEFILE
            //note: for QBASIC compatibility the lowest available file number is returned
            static int32 x;
            for (x=1;x<=gfs_fileno_n;x++) if (gfs_fileno[x]==-1) return x;
            return gfs_fileno_n+1;
        }
        
        
        void gfs_fileno_use(int32 f, int32 i){
            //assumes valid handles
            gfs_fileno[f]=i;
            gfs_file[i].fileno=f;
        }
        
        void gfs_fileno_free(int32 f){//note: called by gfs_free (DO NOT CALL THIS FUNCTION)
            gfs_fileno[f]=-1;
        }
        
        int32 gfs_free(int32 i){
            
            if (!gfs_validhandle(i)) return -2;//invalid handle    
            gfs_file[i].open=0;
            if (gfs_file[i].fileno) gfs_fileno_free(gfs_file[i].fileno);
            gfs_freed.push_back(i);
            return 0;
        }
        
        int32 gfs_close(int32 i){
            static int32 x;
            if (x=gfs_free(i)) return x;
            
            if (gfs_file[i].scrn) return 0; //No further action needed
            gfs_field_buffer_storage[i].clear();
            sync_gfs_field_buffer(i);
            gfs_field_string_storage[i].clear();
            sync_gfs_field_strings(i);
            
            #ifdef GFS_C
                static gfs_file_struct *f;
                f=&gfs_file[i];
                f->file_handle->close();
                delete f->file_handle;
                return 0;
            #endif
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                CloseHandle(f_w->file_handle);
                return 0;
            #endif
            
            return -1;
        }
        
        int64 gfs_lof(int32 i){
            if (!gfs_validhandle(i)) return -2;//invalid handle
            static gfs_file_struct *f;
            f=&gfs_file[i];
            if (f->scrn) return -4;
            #ifdef GFS_C
                f->file_handle->clear();
                if (f->read){
                    static int64 bytes;
                    f->file_handle->seekg(0,ios::end);
                    bytes=f->file_handle->tellg();
                    f->file_handle->seekg(f->pos);
                    return bytes;
                }
                if (f->write){
                    static int64 bytes;
                    f->file_handle->seekp(0,ios::end);
                    bytes=f->file_handle->tellp();
                    f->file_handle->seekp(f->pos);
                    return bytes;
                }
                return -1;
            #endif
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                static int64 bytes;
                *((int32*)&bytes)=GetFileSize(f_w->file_handle,(DWORD*)(((int32*)&bytes)+1));
                if ((bytes&0xFFFFFFFF)==0xFFFFFFFF){
                    if (GetLastError()!=NO_ERROR) return -3;//bad/incorrect file mode
                }
                return bytes;
            #endif
            
            return -1;
        }
        
        int32 gfs_open_com_syntax(qbs *fstr,gfs_file_struct *f){
            //0=not an open com statement
            //-1=syntax error
            //1=valid
            //check if filename is a COM open command
            static int32 x,i;
            static int32 v,c,z;
            f->com_port=0;
            if (fstr->len<=3) return 0;
            if ((fstr->chr[0]&223)!=67) return 0;//"C"/"c"
            if ((fstr->chr[1]&223)!=79) return 0;//"O"/"o"
            if ((fstr->chr[2]&223)!=77) return 0;//"M"/"m"
            v=-1;
            for (i=3;i<fstr->len-1;i++){
                c=fstr->chr[i];
                if (c==58) goto comstatment;
                if ((c<48)||(c>57)) return 0;//not 0-9
                if (v==-1){
                    if (c==48) return 0;//first digit 0
                    v=0;
                }
                v=v*10+(c-48);
            }
            return 0;//no ":"
            comstatment:
            if ((x>=7)||(v<=0)||(v>255)) return -1;//invalid port number (1-255)
            f->com_port=v;
            
            //COM open confirmed
            static qbs *str=NULL;
            if (!str) str=qbs_new(0,0);
            qbs_set(str,qbs_ucase(fstr));
            str->len--;//remove null term.
            
            //set option values to uninitialized
            //---group 1
            f->com_baud_rate=-1;
            f->com_parity=-1;
            f->com_data_bits_per_byte=-1;
            f->com_stop_bits=-1;
            //---group 2
            f->com_rs=-1;
            f->com_bin_asc=-1;
            f->com_asc_lf=-1;
            f->com_cd_x=-1;
            f->com_cs_x=-1;
            f->com_ds_x=-1;
            f->com_op_x=-1;
            
            static int32 str_or_num;
            static int64 strv;
            static int32 stage;
            static int32 com_rb_used,com_tb_used;
            com_rb_used=0; com_tb_used=0; stage=1;
            str_or_num=1; strv=0; v=-1;
            for (i=i+1;i<str->len;i++){ c=str->chr[i];
                if (c!=44){
                    if ((c<48)||((c>57)&&(c<65))||(c>90)) return -1;//invalid character
                    if ((str_or_num==2)&&(c>=65)) return -1;//invalid character
                    if (c<65) str_or_num=2;//ABC->123
                    if ((str_or_num==1)||(stage==4)){//note: stage 4 is interpreted as a string
                        if (strv&0xFF0000) strv=strv|(c<<24); else
                        if (strv&0x00FF00) strv=strv|(c<<16); else
                        if (strv&0x0000FF) strv=strv|(c<<8); else
                        strv=strv=c;
                        if (strv>16777216) return -1;//string option too long (max 3 characters)
                        }else{
                        if ((c>48)&&(c<=57)){
                            if (v==-2) return -1;//leading 0s are invalid
                            if (v==-1) v=0;
                            v=v*10+(c-48);
                            }else{//0
                            if (v==-2) return -1;//leading 0s are invalid
                            if (v==-1) v=-2;//0...
                            if (v>0) v=v*10; 
                        }
                        if (v>2147483647) return -1;//numeric value too large (LONG values only)
                    }
                }//c!=44
                if ((c==44)||(i==str->len-1)){
                    if (v==-2) v=0;
                    //note: v==-1 means omit
                    if (stage==1){
                        if (f->com_baud_rate!=-1) return -1;
                        if (strv) return -1;
                        if (v==0) return -1;
                        if (v==-1) v=300;
                        f->com_baud_rate=v;
                        stage++; goto done_stage;
                    }
                    if (stage==2){
                        if (f->com_parity!=-1) return -1;
                        if (v!=-1) return -1;
                        x=-1;
                        if (strv==78) x=0;//N
                        if (strv==0) x=1;//E*
                        if (strv==69) x=1;//E
                        if (strv==79) x=2;//O
                        if (strv==83) x=3;//S
                        if (strv==77) x=4;//M
                        if (strv==17744) x=5;//PE
                        if (x==-1) return -1;
                        f->com_parity=x;
                        stage++; goto done_stage;
                    }
                    if (stage==3){
                        if (f->com_data_bits_per_byte!=-1) return -1;
                        if (strv) return -1;
                        x=-1;
                        if (v==-1) x=7;
                        if (v==5) x=5;
                        if (v==6) x=6;
                        if (v==7) x=7;
                        if (v==8) x=8;
                        if (x==-1) return -1;
                        f->com_data_bits_per_byte=x;
                        stage++; goto done_stage;
                    }
                    if (stage==4){
                        if (f->com_stop_bits!=-1) return -1;
                        if (v!=-1) return -1;
                        x=-1;
                        if (strv==0){
                            x=10;
                            if (f->com_baud_rate<=110){
                                x=20;
                                if (f->com_data_bits_per_byte==5) x=15;
                            }
                        }//0
                        if (strv==49) x=10;//"1"
                        if (strv==3485233) x=15;//"1.5"
                        if (strv==50) x=20;//"2"
                        if (x==-1) return -1;
                        f->com_stop_bits=x;
                        stage++; goto done_stage;
                    }
                    //stage>4
                    if (!strv) return -1;//all options after 4 require a string
                    if (strv==21330){ if (f->com_rs!=-1) return -1;//RS
                        f->com_rs=1; 
                    goto done_stage;}
                    if (strv==5130562){ if (f->com_bin_asc!=-1) return -1;//BIN
                        f->com_bin_asc=0;
                    goto done_stage;}
                    if (strv==4412225){ if (f->com_bin_asc!=-1) return -1;//ASC
                        f->com_bin_asc=1;
                    goto done_stage;}
                    if (strv==16980){ if (com_tb_used) return -1;//TB
                        com_tb_used=1;
                    goto done_stage;}
                    if (strv==16978){ if (com_rb_used) return -1;//RB
                        com_rb_used=1;
                    goto done_stage;}
                    if (strv==17996){ if (f->com_asc_lf!=-1) return -1;//LF
                        f->com_asc_lf=1;
                    goto done_stage;}
                    if (strv==17475){ if (f->com_cd_x!=-1) return -1;//CD
                        if (v==-1) v=0;
                        if (v>65535) return -1;
                        f->com_cd_x=v;
                    goto done_stage;}
                    if (strv==21315){ if (f->com_cs_x!=-1) return -1;//CS
                        if (v==-1) v=1000;
                        if (v>65535) return -1;
                        f->com_cs_x=v;
                    goto done_stage;}
                    if (strv==21316){ if (f->com_ds_x!=-1) return -1;//DS
                        if (v==-1) v=1000;
                        if (v>65535) return -1;
                        f->com_ds_x=v;
                    goto done_stage;}
                    if (strv==20559){ if (f->com_op_x!=-1) return -1;//OP
                        if (v==-1) v=10000;
                        if (v>65535) return -1;
                        f->com_op_x=v;
                    goto done_stage;}
                    return -1;//invalid option
                    done_stage:
                    str_or_num=1; strv=0; v=-1;
                }
            }//i
            
            //complete omitted options with defaults
            if (f->com_baud_rate==-1) f->com_baud_rate=300;
            if (f->com_parity==-1) f->com_parity=1;
            if (f->com_data_bits_per_byte==-1) f->com_data_bits_per_byte=7;
            if (f->com_stop_bits==-1){
                x=10;
                if (f->com_baud_rate<=110){
                    x=20;
                    if (f->com_data_bits_per_byte==5) x=15;
                }
                f->com_stop_bits=x; 
            }
            if (f->com_bin_asc==-1) f->com_bin_asc=0;
            if (f->com_asc_lf==-1) f->com_asc_lf=0;
            if (f->com_asc_lf==1){
                if (f->com_bin_asc==0) f->com_asc_lf=0;
            }
            if (f->com_rs==-1) f->com_rs=0;
            if (f->com_cd_x==-1) f->com_cd_x=0;
            if (f->com_cs_x==-1) f->com_cs_x=1000;
            if (f->com_ds_x==-1) f->com_ds_x=1000;
            if (f->com_op_x==-1){
                x=f->com_cd_x*10; z=f->com_ds_x*10;
                if (z>x) x=z;
                if (x>65535) x=65535;
                f->com_op_x=x;
            }
            
        }
        
        
        int32 gfs_open(qbs *filename,int32 access,int32 restrictions, int32 how){
            //filename - an OS compatible filename (doesn't need to be NULL terminated)
            //access - 1=read, 2=write, 3=read and write
            //restrictions - 1=others cannot read, 2=others cannot write, 3=others cannot read or write(exclusive access)
            //how - 1=create(if it doesn't exist), 2=create(if it doesn't exist) & truncate
            //      3=create(if it doesn't exist)+undefined access[get whatever access is available]
            static int32 i,x,x2,x3,e;
            static qbs *filenamez=NULL;
            static gfs_file_struct *f;
            
            if (!filenamez) filenamez=qbs_new(0,0);
            qbs_set(filenamez,qbs_add(filename,qbs_new_txt_len("\0",1)));
            
            i=gfs_new();
            f=&gfs_file[i];
            
            int32 v1;                
            unsigned char *c1=filename->chr;                
            v1=*c1;                
            if (v1==83||v1==115) {  //S                
                c1++;                
                v1=*c1;                
                if (v1==67||v1==99) {  //C                
                    c1++;                
                    v1=*c1;                
                    if (v1==82||v1==114) {  //R                
                        c1++;                
                        v1=*c1;                
                        if (v1==78||v1==110) {  //N                
                            c1++;                
                            v1=*c1;                
                            if (v1==58) {  //:                
                                f->scrn=1;                
                                return i;                
                            };                
                        };                
                    };                
                };                
            };
            
            
            if (access&1) f->read=1;
            if (access&2) f->write=1;
            if (restrictions&1) f->lock_read=1;
            if (restrictions&2) f->lock_write=1;
            f->pos=0;
            
            //check for OPEN COM syntax
            if (x=gfs_open_com_syntax(filenamez,f)){
                if (x==-1){gfs_free(i); return -11;}//-11 bad file name
                //note: each GFS implementation will handle COM communication differently
            }
            
            
            #ifdef GFS_C
                //note: GFS_C ignores restrictions/locking
                f->file_handle=new fstream();
                //attempt as if it exists:
                if (how==2){
                    //with truncate
                    if (access==1) f->file_handle->open(fixdir(filenamez),ios::in|ios::binary|ios::trunc);
                    if (access==2) f->file_handle->open(fixdir(filenamez),ios::out|ios::binary|ios::trunc);
                    if (access==3) f->file_handle->open(fixdir(filenamez),ios::in|ios::out|ios::binary|ios::trunc);
                    }else{
                    //without truncate
                    if (access==1) f->file_handle->open(fixdir(filenamez),ios::in|ios::binary);
                    if (access==2) f->file_handle->open(fixdir(filenamez),ios::out|ios::binary|ios::app);
                    if (access==3) f->file_handle->open(fixdir(filenamez),ios::in|ios::out|ios::binary);
                }
                if (how){
                    if (!f->file_handle->is_open()){//couldn't open file, so attempt creation
                        f->file_handle_o=new ofstream();
                        f->file_handle_o->open(fixdir(filenamez),ios::out);
                        if (f->file_handle_o->is_open()){//created new file
                            f->file_handle_o->close();
                            //retry open 
                            f->file_handle->clear();
                            if (how==2){
                                //with truncate
                                if (access==1) f->file_handle->open(fixdir(filenamez),ios::in|ios::binary|ios::trunc);
                                if (access==2) f->file_handle->open(fixdir(filenamez),ios::out|ios::binary|ios::trunc);
                                if (access==3) f->file_handle->open(fixdir(filenamez),ios::in|ios::out|ios::binary|ios::trunc);
                                }else{
                                //without truncate
                                if (access==1) f->file_handle->open(fixdir(filenamez),ios::in|ios::binary);
                                if (access==2) f->file_handle->open(fixdir(filenamez),ios::out|ios::binary|ios::app);
                                if (access==3) f->file_handle->open(fixdir(filenamez),ios::in|ios::out|ios::binary);
                            }
                        }
                        delete f->file_handle_o;
                    }
                }//how 
                if (!f->file_handle->is_open()){//couldn't open file
                    delete f->file_handle;
                    gfs_free(i);
                    if (how) return -11;//Bad file name
                    return -5;//File not found
                }
                //file opened successfully
                f->open=1;
                return i;
            #endif
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                x=0;
                if (access&1)x|=GENERIC_READ;
                if (access&2)x|=GENERIC_WRITE;
                x2=FILE_SHARE_READ|FILE_SHARE_WRITE;
                if (restrictions&1)x2^=FILE_SHARE_READ;
                if (restrictions&2)x2^=FILE_SHARE_WRITE;
                
                if (f->com_port){//open a com port
                    static qbs *portname=NULL; if (!portname) portname=qbs_new(0,0);
                    qbs_set(portname,qbs_add(qbs_new_txt("CO"),qbs_str(f->com_port)));
                    qbs_set(portname,qbs_add(portname,qbs_new_txt_len(":\0",2)));
                    portname->chr[2]=77;//replace " " with "M"
                    f_w->file_handle=CreateFile((char*)portname->chr,x,0,NULL,OPEN_EXISTING,0,NULL);
                    if (f_w->file_handle==INVALID_HANDLE_VALUE){gfs_free(i); return -8;}//device unavailable
                    static DCB cs;
                    ZeroMemory(&cs,sizeof(DCB));
                    cs.DCBlength=sizeof(DCB);
                    if (!GetCommState(f_w->file_handle,&cs)){CloseHandle(f_w->file_handle); gfs_free(i); return -8;}//device unavailable
                    static COMMTIMEOUTS ct;
                    ZeroMemory(&ct,sizeof(COMMTIMEOUTS));
                    /*dump port state and return "file not found" (used for debugging only)
                        if (!GetCommTimeouts(f_w->file_handle,&ct)){CloseHandle(f_w->file_handle); gfs_free(i); return -8;}//device unavailable
                        ofstream mydump("f:\\comdump.bin");
                        mydump.write((char*)&cs,sizeof(cs));
                        mydump.write((char*)&ct,sizeof(ct));
                        mydump.close();
                        CloseHandle(f_w->file_handle); gfs_free(i);
                        return -4;
                    */
                    cs.BaudRate=f->com_baud_rate;
                    x=f->com_stop_bits;
                    if (x==10) x2=0;
                    if (x==15) x2=1;
                    if (x==20) x2=2;
                    cs.StopBits=x2;
                    cs.ByteSize=f->com_data_bits_per_byte; 
                    x=f->com_parity;
                    if (x==0) x2=0;
                    if (x==1) x2=2;
                    if (x==2) x2=1;
                    if (x==3) x2=4;
                    if (x==4) x2=3;
                    //if (x==5) x2=... ***"PE" will be supported later***
                    //0-4=None,Odd,Even,Mark,Space  
                    cs.Parity=x2; if (x2==0) cs.fParity=0; else cs.fParity=1;
                    if (f->com_rs) cs.fRtsControl=RTS_CONTROL_DISABLE;
                    if (f->com_bin_asc==0) cs.fBinary=1; else cs.fBinary=0;
                    cs.EofChar=26;
                    if(!SetCommState(f_w->file_handle,&cs)){CloseHandle(f_w->file_handle); gfs_free(i); return -8;}//device unavailable
                    if (f->com_ds_x==0){
                        //A value of MAXDWORD, combined with zero values for both the ReadTotalTimeoutConstant and ReadTotalTimeoutMultiplier members, specifies that the read operation is to return immediately with the characters that have already been received, even if no characters have been received.
                        ct.ReadIntervalTimeout=MAXDWORD;
                        ct.ReadTotalTimeoutMultiplier=0;
                        ct.ReadTotalTimeoutConstant=0;
                        }else{
                        ct.ReadIntervalTimeout=0;
                        ct.ReadTotalTimeoutMultiplier=0;
                        ct.ReadTotalTimeoutConstant=f->com_ds_x;
                    }
                    ct.WriteTotalTimeoutMultiplier=0;
                    ct.WriteTotalTimeoutConstant=f->com_cs_x;
                    if (!SetCommTimeouts(f_w->file_handle,&ct)){CloseHandle(f_w->file_handle); gfs_free(i); return -8;}//device unavailable
                    f->open=1;
                    return i;
                }
                
                /*
                    #define CREATE_NEW          1
                    #define CREATE_ALWAYS       2
                    #define OPEN_EXISTING       3
                    #define OPEN_ALWAYS         4
                    #define TRUNCATE_EXISTING   5
                */ 
                x3=OPEN_EXISTING;
                if (how) x3=OPEN_ALWAYS;
                undefined_retry: 
                f_w->file_handle=CreateFile(fixdir(filenamez),x,x2,NULL,x3,FILE_ATTRIBUTE_NORMAL,NULL);
                if (f_w->file_handle==INVALID_HANDLE_VALUE){
                    
                    if (how==3){
                        //attempt read access only
                        x=GENERIC_READ;
                        f->read=1;
                        f->write=0;
                        how++;
                        goto undefined_retry;
                    }
                    
                    if (how==4){
                        //attempt write access only
                        x=GENERIC_WRITE;
                        f->read=0;
                        f->write=1;
                        how++;
                        goto undefined_retry;
                    }
                    
                    gfs_free(i);
                    e=GetLastError();
                    if (e==3) return -6;
                    if ((e==4)||(e==29)||(e==30)) return -9;
                    if ((e==5)||(e==19)||(e==33)||(e==32)) return -7;
                    if ((e==15)||(e==21)) return -8;
                    if (e==2) return -5;
                    //showvalue(e);
                    return -5;//assume (2)
                }//invalid handle
                
                if (how==2){
                    //truncate file if size is not 0
                    static DWORD GetFileSize_low,GetFileSize_high;
                    GetFileSize_low=GetFileSize(f_w->file_handle,&GetFileSize_high);
                    if (GetFileSize_low||GetFileSize_high){
                        CloseHandle(f_w->file_handle);
                        x3=TRUNCATE_EXISTING;
                        f_w->file_handle=CreateFile(fixdir(filenamez),x,x2,NULL,x3,FILE_ATTRIBUTE_NORMAL,NULL);
                        
                        if (f_w->file_handle==INVALID_HANDLE_VALUE){
                            gfs_free(i);
                            e=GetLastError();
                            if (e==3) return -6;
                            if ((e==4)||(e==29)||(e==30)) return -9;
                            if ((e==5)||(e==19)||(e==33)||(e==32)) return -7;
                            if ((e==15)||(e==21)) return -8;
                            if (e==2) return -5;
                            //showvalue(e);
                            return -5;//assume (2)
                        }//invalid handle
                        
                    }
                }//how==2
                f->open=1;
                return i;
            #endif
            
            
            return -1;//non-specific fail
            
        }
        
        int32 gfs_setpos(int32 i, int64 position){
            if (!gfs_validhandle(i)) return -2;//invalid handle
            if (position<0) return -4;//illegal function call
            
            static gfs_file_struct *f;
            f=&gfs_file[i];
            
            #ifdef GFS_C
                if (f->read){
                    f->file_handle->clear();
                    f->file_handle->seekg(position);
                }
                if (f->write){
                    f->file_handle->clear();
                    f->file_handle->seekp(position);
                }
                f->pos=position;
                if (f->pos<=gfs_lof(i)){
                    f->eof_passed=0;
                    f->eof_reached=0;
                }
                return 0;
            #endif
            
            
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                if (SetFilePointer(f_w->file_handle,(int32)position,(long*)(((int32*)&position)+1),FILE_BEGIN)==0xFFFFFFFF){/*Note that it is not an error to set the file pointer to a position beyond the end of the file. The size of the file does not increase until you call the SetEndOfFile, WriteFile, or WriteFileEx function.*/
                    if (GetLastError()!=NO_ERROR){
                        return -3;//bad file mode
                    }
                }
                f->pos=position;
                if (f->pos<=gfs_lof(i)){
                    f->eof_passed=0;
                    f->eof_reached=0;
                }
                return 0;
            #endif
            
            return -1;
        }
        
        int64 gfs_getpos(int32 i){
            if (!gfs_validhandle(i)) return -2;//invalid handle
            static gfs_file_struct *f;
            f=&gfs_file[i];
            return f->pos;
        }
        
        int32 gfs_write(int32 i,int64 position,uint8 *data,int64 size){
            if (!gfs_validhandle(i)) return -2;//invalid handle
            static int32 e;
            static gfs_file_struct *f;
            f=&gfs_file[i];
            if (!f->write) return -3;//bad file mode
            if (size<0) return -4;//illegal function call
            static int32 x;
            if (position!=-1){
                if (x=gfs_setpos(i,position)) return x;//(pass on error)
            }
            
            #ifdef GFS_C
                f->file_handle->clear();
                f->file_handle->write((char*)data,size);
                if (f->file_handle->bad()){
                    return -7;//assume: permission denied
                }
                f->pos+=size;
                return 0;
            #endif
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                static uint32 size2;
                static int64 written=0;
                while(size){
                    if (size>4294967295){
                        size2=4294967295;
                        size-=4294967295;
                        }else{
                        size2=size; size=0;
                    }
                    if (!WriteFile(f_w->file_handle,data,size2,(unsigned long*)&written,NULL)){
                        e=GetLastError();
                        if ((e==5)||(e==33)) return -7;//permission denied
                        return -9;//assume: path/file access error
                    }
                    data+=written; f->pos+=written;
                    if (written!=size2) return -1;
                }
                return 0;
            #endif
            
            return -1;
        }
        
        
        int64 gfs_read_bytes_value;
        int64 gfs_read_bytes(){
            return gfs_read_bytes_value;
        }
        
        int32 gfs_read(int32 i,int64 position,uint8 *data,int64 size){
            gfs_read_bytes_value=0;
            if (!gfs_validhandle(i)) return -2;//invalid handle
            static int32 e;
            static gfs_file_struct *f;
            f=&gfs_file[i];
            if (!f->read) return -3;//bad file mode
            if (size<0) return -4;//illegal function call
            static int32 x;
            if (position!=-1){
                if (x=gfs_setpos(i,position)) return x;//(pass on error)
            }
            
            #ifdef GFS_C
                f->file_handle->clear();
                f->file_handle->read((char*)data,size); 
                if (f->file_handle->bad()){//note: 'eof' also sets the 'fail' flag, so only the the 'bad' flag is checked
                    return -7;//assume: permission denied
                }
                static int64 bytesread;
                bytesread=f->file_handle->gcount();
                gfs_read_bytes_value=bytesread;
                f->pos+=bytesread;
                if (bytesread<size){
                    memset(data+bytesread,0,size-bytesread);
                    f->eof_passed=1;
                    return -10;
                }
                f->eof_passed=0;
                return 0;
            #endif
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                static uint32 size2;
                static int64 bytesread=0;
                while(size){
                    if (size>4294967295){
                        size2=4294967295;
                        size-=4294967295;
                        }else{
                        size2=size; size=0;
                    }
                    
                    if (ReadFile(f_w->file_handle,data,size2,(unsigned long*)&bytesread,NULL)){
                        data+=bytesread; f->pos+=bytesread; gfs_read_bytes_value+=bytesread;
                        if (bytesread!=size2){   
                            ZeroMemory(data,size+(size2-bytesread));//nullify remaining buffer
                            f->eof_passed=1; return -10;
                        }//eof passed
                        }else{
                        //error
                        e=GetLastError();
                        if ((e==5)||(e==33)) return -7;//permission denied
                        //showvalue(e);
                        return -9;//assume: path/file access error
                    }
                }
                f->eof_passed=0;
                return 0;
            #endif
            
            return -1;
        }
        
        int32 gfs_eof_reached(int32 i){
            if (!gfs_validhandle(i)) return -2;//invalid handle
            if (gfs_getpos(i)>=gfs_lof(i)) return 1;
            return 0;
        }
        
        int32 gfs_eof_passed(int32 i){
            if (!gfs_validhandle(i)) return -2;//invalid handle
            static gfs_file_struct *f;
            f=&gfs_file[i];
            if (f->eof_passed) return 1;
            return 0;
        }
        
        int32 gfs_lock(int32 i,int64 offset_start,int64 offset_end){
            //if offset_start==-1, 'from beginning' (typically offset 0) is assumed
            //if offset_end==-1, 'to end/infinity' is assumed
            //range is inclusive of start and end
            if (!gfs_validhandle(i)) return -2;//invalid handle
            static gfs_file_struct *f;
            f=&gfs_file[i];
            
            if (offset_start==-1) offset_start=0;
            if (offset_start<0) return -4;//illegal function call
            if (offset_end<-1) return -4;//illegal function call
            //note: -1 equates to highest uint64 value (infinity)
            //      All other negative end values are illegal
            
            #ifdef GFS_C
                return 0;
            #endif
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                uint64 bytes;
                bytes=offset_end;
                if (bytes!=-1) bytes=bytes-offset_start+1;
                if (!LockFile(f_w->file_handle,*((DWORD*)(&offset_start)),*(((DWORD*)(&offset_start))+1),*((DWORD*)(&bytes)),*(((DWORD*)(&bytes))+1))){
                    //failed
                    static int32 e;
                    e=GetLastError();
                    if ((e==5)||(e==33)) return -7;//permission denied
                    //showvalue(e);
                    return -9;//assume: path/file access error
                }
                return 0;
            #endif
            
            return -1;
        }
        
        int32 gfs_unlock(int32 i,int64 offset_start,int64 offset_end){
            //if offset_start==-1, 'from beginning' (typically offset 0) is assumed
            //if offset_end==-1, 'to end/infinity' is assumed
            //range is inclusive of start and end
            if (!gfs_validhandle(i)) return -2;//invalid handle
            static gfs_file_struct *f;
            f=&gfs_file[i];
            
            if (offset_start==-1) offset_start=0;
            if (offset_start<0) return -4;//illegal function call
            if (offset_end<-1) return -4;//illegal function call
            //note: -1 equates to highest uint64 value (infinity)
            //      All other negative end values are illegal
            
            #ifdef GFS_C
                return 0;
            #endif
            
            #ifdef GFS_WINDOWS
                static gfs_file_win_struct *f_w;
                f_w=&gfs_file_win[i];
                uint64 bytes;
                bytes=offset_end;
                if (bytes!=-1) bytes=bytes-offset_start+1;
                if (!UnlockFile(f_w->file_handle,*((DWORD*)(&offset_start)),*(((DWORD*)(&offset_start))+1),*((DWORD*)(&bytes)),*(((DWORD*)(&bytes))+1))){
                    //failed
                    static int32 e;
                    e=GetLastError(); 
                    if ((e==5)||(e==33)||(e==158)) return -7;//permission denied
                    //showvalue(e);
                    return -9;//assume: path/file access error
                }
                return 0;
            #endif
            
            return -1;
        }
        
        void sub_lock(int32 i,int64 start,int64 end,int32 passed){
            if (new_error) return;
            if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            static gfs_file_struct *gfs;
            gfs=&gfs_file[i];
            
            //If the file has been opened for sequential input or output, LOCK and UNLOCK affect the entire file, regardless of the range specified by start& and end&.
            if (gfs->type>2) passed=0;
            
            if (passed&1){
                start--;
                if (start<0){error(5); return;}
                if (gfs->type==1) start*=gfs->record_length;
                }else{
                start=-1;
            }
            
            if (passed&2){
                end--;
                if (end<0){error(5); return;}
                if (gfs->type==1) end=end*gfs->record_length+gfs->record_length-1;
                }else{
                end=start;
                if (gfs->type==1) end=start+gfs->record_length-1;
                if (!(passed&1)) end=-1;
            }
            
            
            int32 e;
            e=gfs_lock(i,start,end);
            if (e){
                if (e==-2){error(258); return;}//invalid handle
                if (e==-4){error(5); return;}//illegal function call
                if (e==-7){error(70); return;}//permission denied
                error(75); return;//assume[-9]: path/file access error
            }
            
        }
        
        void sub_unlock(int32 i,int64 start,int64 end,int32 passed){
            if (new_error) return;
            if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            static gfs_file_struct *gfs;
            gfs=&gfs_file[i];
            
            //If the file has been opened for sequential input or output, LOCK and UNLOCK affect the entire file, regardless of the range specified by start& and end&.
            if (gfs->type>2) passed=0;
            
            if (passed&1){
                start--;
                if (start<0){error(5); return;}
                if (gfs->type==1) start*=gfs->record_length;
                }else{
                start=-1;
            }
            
            if (passed&2){
                end--;
                if (end<0){error(5); return;}
                if (gfs->type==1) end=end*gfs->record_length+gfs->record_length-1;
                }else{
                end=start;
                if (gfs->type==1) end=start+gfs->record_length-1;
                if (!(passed&1)) end=-1;
            }
            
            int32 e;
            e=gfs_unlock(i,start,end);
            if (e){
                if (e==-2){error(258); return;}//invalid handle
                if (e==-4){error(5); return;}//illegal function call
                if (e==-7){error(70); return;}//permission denied
                error(75); return;//assume[-9]: path/file access error
            }
            
        }
// ---- lines 26062,26292
        static int32 field_failed=1;
        static int32 field_fileno;
        static int32 field_totalsize;
        static int32 field_maxsize;
        
        void field_new(int32 fileno){
            field_failed=1;
            if (new_error) return;
            //validate file
            static int32 i;
            static gfs_file_struct *gfs;
            i=fileno;
            if (i<0){error(54); return;}//bad file mode (TCP/IP exclusion)
            if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            gfs=&gfs_file[i];
            if (gfs->type!=1){error(54); return;}//Bad file mode (note: must have RANDOM access)
            //set global variables for field_add
            field_fileno=fileno;
            field_totalsize=0;
            field_maxsize=gfs->record_length;
            field_failed=0;
            return;
        }
        
        
        void field_update(int32 fileno){
            
            //validate file
            static int32 i;
            static gfs_file_struct *gfs;
            i=fileno;
            if (i<0){exit(7701);}//bad file mode (TCP/IP exclusion)
            if (gfs_fileno_valid(i)!=1){exit(7702);}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            gfs=&gfs_file[i];
            if (gfs->type!=1){exit(7703);}//Bad file mode (note: must have RANDOM access)
            
            static qbs *str;
            for (i=0;i<gfs->field_strings_n;i++){
                str=gfs->field_strings[i];
                if (!str) exit(7704);
                
                //fix length if necessary
                if (str->len!=str->field->size){
                    if (str->len>str->field->size) str->len=str->field->size; else qbs_set(str,qbs_new(str->field->size,1));
                }
                
                //copy data from field into string
                memmove(str->chr,gfs->field_buffer+str->field->offset,str->field->size);
                
            }//i
            
        }
        
        void lrset_field(qbs *str){
            //validate file
            static int32 i;
            static gfs_file_struct *gfs;
            i=str->field->fileno;
            if (gfs_fileno_valid(i)!=1) goto remove;
            i=gfs_fileno[i];//convert fileno to gfs index
            
            gfs=&gfs_file[i];
            if (gfs->type!=1) goto remove;
            //check file ID
            if (gfs->id!=str->field->fileid) goto remove;
            
            //store in field buffer, padding with spaces or truncating data if necessary
            if (str->field->size<=str->len){
                
                memmove(gfs->field_buffer+str->field->offset,str->chr,str->field->size);
                }else{
                memmove(gfs->field_buffer+str->field->offset,str->chr,str->len);
                memset(gfs->field_buffer+str->field->offset+str->len,32,str->field->size-str->len);
            }
            
            //update field strings for this file
            field_update(str->field->fileno);
            
            return;
            remove:;
            delete str->field;
            str->field=NULL;
        }
        
        void field_free(qbs* str){
            
            //validate file
            static int32 i;
            static gfs_file_struct *gfs;
            i=str->field->fileno;
            if (gfs_fileno_valid(i)!=1) goto remove;
            i=gfs_fileno[i];//convert fileno to gfs index
            gfs=&gfs_file[i];
            if (gfs->type!=1) goto remove;
            //check file ID
            if (gfs->id!=str->field->fileid) goto remove;
            
            //remove from string list
            static qbs *str2;
            for (i=0;i<gfs->field_strings_n;i++){
                str2=gfs->field_strings[i];
                if (str==str2){//match found
                    std::vector<qbs*> &fields=gfs_field_string_storage[gfs_fileno[str->field->fileno]];
                    fields.erase(fields.begin()+i);
                    sync_gfs_field_strings(gfs_fileno[str->field->fileno]);
                    goto remove;
                }
            }//i
            
            remove:
            delete str->field;
            str->field=NULL;
        }
        
        void field_add(qbs *str,int64 size){
            if (field_failed) return;
            if (new_error) goto fail;
            if (size<0){error(5); goto fail;}
            if ((field_totalsize+size)>field_maxsize){error(50); goto fail;}
            
            //revalidate file
            static int32 i;
            static gfs_file_struct *gfs;
            i=field_fileno;
            //TCP/IP exclusion (reason: multi-reading from same TCP/IP position would require a more complex implementation)
            if (i<0){error(54); goto fail;}//bad file mode
            if (gfs_fileno_valid(i)!=1){error(52); goto fail;}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            gfs=&gfs_file[i];
            if (gfs->type!=1){error(54); goto fail;}//Bad file mode (note: must have RANDOM access)
            
            //1) Remove str from any previous FIELD allocations
            if (str->field) field_free(str);
            
            //2) Setup qbs field info
            str->field=new(std::nothrow) qbs_field{};
            if (!str->field){error(518); goto fail;}
            str->field->fileno=field_fileno;
            str->field->fileid=gfs->id;
            str->field->size=size;
            str->field->offset=field_totalsize;
            
            //3) Add str to qbs list of gfs
            gfs_field_string_storage[i].push_back(str);
            sync_gfs_field_strings(i);
            
            //4) Update field strings for this file
            field_update(field_fileno);
            
            field_totalsize+=size;
            return;
            fail:
            field_failed=1;
            return;
        }
        
        void field_get(int32 fileno,int64 offset,int32 passed){
            if (new_error) return;
            
            //validate file
            static int32 i;
            static gfs_file_struct *gfs;
            i=fileno;
            if (i<0){error(54); return;}//bad file mode (TCP/IP exclusion)
            if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            gfs=&gfs_file[i];
            if (gfs->type!=1){error(54); return;}//Bad file mode (note: must have RANDOM access)
            
            if (!gfs->read){error(75); return;}//Path/file access error
            
            if (passed){
                offset--;
                if (offset<0){error(63); return;}//Bad record number
                offset*=gfs->record_length;
                }else{
                offset=-1;
            }
            
            static int32 e;
            e=gfs_read(i,offset,gfs->field_buffer,gfs->record_length);
            if (e){
                if (e!=-10){//note: on eof, unread buffer area becomes NULL
                    if (e==-2){error(258); return;}//invalid handle
                    if (e==-3){error(54); return;}//bad file mode
                    if (e==-4){error(5); return;}//illegal function call
                    if (e==-7){error(70); return;}//permission denied
                    error(75); return;//assume[-9]: path/file access error
                }
            }
            
            field_update(fileno);
            
        }
        
        void field_put(int32 fileno,int64 offset,int32 passed){
            if (new_error) return;
            
            //validate file
            static int32 i;
            static gfs_file_struct *gfs;
            i=fileno;
            if (i<0){error(54); return;}//bad file mode (TCP/IP exclusion)
            if (gfs_fileno_valid(i)!=1){error(52); return;}//Bad file name or number
            i=gfs_fileno[i];//convert fileno to gfs index
            gfs=&gfs_file[i];
            if (gfs->type!=1){error(54); return;}//Bad file mode (note: must have RANDOM access)
            
            if (!gfs->write){error(75); return;}//Path/file access error
            
            if (passed){
                offset--;
                if (offset<0){error(63); return;}//Bad record number
                offset*=gfs->record_length;
                }else{
                offset=-1;
            }
            
            static int32 e;
            e=gfs_write(i,offset,gfs->field_buffer,gfs->record_length);
            if (e){
                if (e==-2){error(258); return;}//invalid handle
                if (e==-3){error(54); return;}//bad file mode
                if (e==-4){error(5); return;}//illegal function call
                if (e==-7){error(70); return;}//permission denied
                error(75); return;//assume[-9]: path/file access error
            }
            
        }
