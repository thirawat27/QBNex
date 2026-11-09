// Verbatim legacy code from internal/c/libqb.cpp (commit ce58afd), used by tests/diff/diff_core.cpp.
// Do not edit: it is the reference the Rust port is compared against.

//number limits
const char *range_int64_max[]={"9223372036854775807"};//19 digits
const char *range_int64_neg_max[]={"9223372036854775808"};//19 digits
const char *range_uint64_max[]={"18446744073709551615"};//20 digits
const char *range_float_max[]=    {"17976931348623157"};//17 digits

//universal number representation
uint16 n_digits;
uint8 n_digit[256];
int64 n_exp;//if 0, there is one digit in front of the decimal place
uint8 n_neg;//if 1, the number is negative
uint8 n_hex;//if 1, the digits are in hexidecimal and n_exp should be ignored
            //if 2, the digits are in octal and n_exp should be ignored
            //if 3, the digits are in binary and n_exp should be ignored
            //(consider revising variable name n_hex)

int32 n_roundincrement(){
    static int32 i,i2,i3;
    if (n_digits==0) return 0;
    if (n_digits>(n_exp+1)){//numbers exist after the decimal point
        i=n_digit[n_exp+1]-48;
        if (i>=5) return 1;
    }
    return 0;
}

long double n_float_value;
int32 n_float(){
    //return value: Bit 0=successful
    //data
    static uint8 built[256];
    static int64 value;
    uint64 uvalue;
    static int32 i,i2,i3;
    static uint8 *max;
    max=(uint8*)range_float_max[0];
    n_float_value=0; value=0; uvalue=0;
    if (n_digits==0) return 1;
    //hex?
    if (n_hex==1){
        if (n_digits>16) return 0;
        for (i=0;i<n_digits;i++){
            i2=n_digit[i];
            if ((i2>=48)&&(i2<=57)) i2-=48;
            if ((i2>=65)&&(i2<=70)) i2-=55;
            if ((i2>=97)&&(i2<=102)) i2-=87;
            value<<=4;
            value|=i2;
        }
        n_float_value=value;
        return 1;
    }
    //oct?
    if (n_hex==2){
        if (n_digits>=22){
            if ((n_digits>22)||(n_digit[0]>49)) return 0;
        }
        for (i=0;i<n_digits;i++){
            i2=n_digit[i]-48;
            value<<=3;
            value|=i2;
        }
        n_float_value=value;
        return 1;
    }
    //bin?
    if (n_hex==3){
        if (n_digits>64) return 0;
        for (i=0;i<n_digits;i++){
            i2=n_digit[i]-48;
            value<<=1;
            value|=i2;
        }
        n_float_value=value;
        return 1;
    }
    
    //max range check (+-1.7976931348623157E308)
    if (n_exp>308)return 0;//overflow
    if (n_exp==308){
        i2=n_digits; if (i2>17) i2=17;
        for (i=0;i<i2;i++){
            if (n_digit[i]>max[i]) return 0;//overflow
            if (n_digit[i]<max[i]) break;
        }
    }
    //too close to 0?
    if (n_exp<-324) return 1;
    //read & return value (via C++ function)
    //build number
    i=0;
    if (n_neg){built[i]=45; i++;}//-
    built[i]=n_digit[0]; i++;
    built[i]=46; i++;//.
    if (n_digits==1){
        built[i]=48; i++;//0
        }else{
        i3=n_digits; if (i3>17) i3=17;
        for (i2=1;i2<i3;i2++){
            built[i]=n_digit[i2]; i++;
        }
    }
    built[i]=69; i++;//E
    #ifdef QBNex_WINDOWS
        i2=sprintf((char*)&built[i],"%I64i",n_exp);
        #else
        i2=sprintf((char*)&built[i],"%lli",n_exp);
    #endif
    i=i+i2;
    built[i]=0;//NULL terminate for sscanf
    
    static double sscanf_fix;
    sscanf((char*)&built[0],"%lf",&sscanf_fix);
    n_float_value=sscanf_fix;
    
    return 1;
}

int64 n_int64_value;
int32 n_int64(){
    //return value: Bit 0=successful
    //data
    static int64 value;
    uint64 uvalue;
    static int32 i,i2;
    static uint8 *max; static uint8 *neg_max;
    static int64 v0=build_int64(0x80000000,0x00000000);
    static int64 v1=build_int64(0x7FFFFFFF,0xFFFFFFFF);
    max=(uint8*)range_int64_max[0]; neg_max=(uint8*)range_int64_neg_max[0];
    n_int64_value=0; value=0; uvalue=0;
    if (n_digits==0) return 1;
    //hex
    if (n_hex==1){
        if (n_digits>16) return 0;
        for (i=0;i<n_digits;i++){
            i2=n_digit[i];
            if ((i2>=48)&&(i2<=57)) i2-=48;
            if ((i2>=65)&&(i2<=70)) i2-=55;
            if ((i2>=97)&&(i2<=102)) i2-=87;
            value<<=4;
            value|=i2;
        }
        n_int64_value=value;
        return 1;
    }
    //oct
    if (n_hex==2){
        if (n_digits>=22){
            
            if ((n_digits>22)||(n_digit[0]>49)) return 0;
        }
        for (i=0;i<n_digits;i++){
            i2=n_digit[i]-48;
            value<<=3;
            value|=i2;
        }
        n_int64_value=value;
        return 1;
    }
    //bin
    if (n_hex==3){
        if (n_digits>64) return 0;
        for (i=0;i<n_digits;i++){
            i2=n_digit[i]-48;
            value<<=1;
            value|=i2;
        }
        n_int64_value=value;
        return 1;
    }
    
    //range check: int64 (-9,223,372,036,854,775,808 to 9,223,372,036,854,775,807)
    if (n_exp>18)return 0;//overflow
    if (n_exp==18){
        i2=n_digits; if (i2>19) i2=19;//only scan integeral digits
        for (i=0;i<i2;i++){
            if (n_neg){
                if (n_digit[i]>neg_max[i]) return 0;//overflow
                if (n_digit[i]<neg_max[i]) break;
                }else{
                if (n_digit[i]>max[i]) return 0;//overflow
                if (n_digit[i]<max[i]) break;
            }
        }
    }
    //calculate integeral value
    i2=n_digits; if (i2>(n_exp+1)) i2=n_exp+1;
    for (i=0;i<(n_exp+1);i++){
        uvalue*=10;
        if (i<i2) uvalue=uvalue+(n_digit[i]-48);
    }
    if (n_neg){
        value=-uvalue;
        }else{
        value=uvalue;
    }
    //apply rounding
    if (n_roundincrement()){
        if (n_neg){
            if (value==v0) return 0;
            value--;
            }else{
            if (value==v1) return 0;
            value++;
        }
    }
    //return value
    n_int64_value=value;
    return 1;
}

uint64 n_uint64_value;
int32 n_uint64(){
    //return value: Bit 0=successful
    //data
    static int64 value;
    uint64 uvalue;
    static int32 i,i2;
    static uint8 *max;
    static int64 v0=build_uint64(0xFFFFFFFF,0xFFFFFFFF);
    max=(uint8*)range_uint64_max[0];
    n_uint64_value=0; value=0; uvalue=0;
    if (n_digits==0) return 1;
    //hex
    if (n_hex==1){
        if (n_digits>16) return 0;
        for (i=0;i<n_digits;i++){
            i2=n_digit[i];
            if ((i2>=48)&&(i2<=57)) i2-=48;
            if ((i2>=65)&&(i2<=70)) i2-=55;
            if ((i2>=97)&&(i2<=102)) i2-=87;
            uvalue<<=4;
            uvalue|=i2;
        }
        n_uint64_value=uvalue;
        return 1;
    }
    //oct
    if (n_hex==2){
        if (n_digits>=22){
            if ((n_digits>22)||(n_digit[0]>49)) return 0;
        }
        for (i=0;i<n_digits;i++){
            i2=n_digit[i]-48;
            uvalue<<=3;
            uvalue|=i2;
        }
        n_uint64_value=uvalue;
        return 1;
    }
    //bin
    if (n_hex==3){
        if (n_digits>64) return 0;
        for (i=0;i<n_digits;i++){
            i2=n_digit[i]-48;
            uvalue<<=1;
            uvalue|=i2;
        }
        n_uint64_value=uvalue;
        return 1;
    }
    
    //negative?
    if (n_neg){
        if (n_exp>=0) return 0;//cannot return a negative number!
    }
    //range check: int64 (0 to 18446744073709551615)
    if (n_exp>19)return 0;//overflow
    if (n_exp==19){
        i2=n_digits; if (i2>20) i2=20;//only scan integeral digits
        for (i=0;i<i2;i++){ 
            if (n_digit[i]>max[i]) return 0;//overflow
            if (n_digit[i]<max[i]) break; 
        }
    }
    //calculate integeral value
    i2=n_digits; if (i2>(n_exp+1)) i2=n_exp+1;
    for (i=0;i<(n_exp+1);i++){
        uvalue*=10;
        if (i<i2) uvalue=uvalue+(n_digit[i]-48);
    }
    //apply rounding
    if (n_roundincrement()){
        if (n_neg){
            return 0;
            }else{
            if (uvalue==v0) return 0;
            uvalue++;
        }
    }
    //return value
    n_uint64_value=uvalue;
    
    
    
    return 1;
}

int32 n_inputnumberfromdata(uint8 *data,ptrszint *data_offset,ptrszint data_size){
    //return values:
    //0=success!
    //1=Overflow
    //2=Out of DATA
    //3=Syntax error
    //note: when read fails the data_offset MUST be restored to its old position
    
    //data
    static int32 i,i2;
    static int32 step,c;
    static int32 exponent_digits;
    static uint8 negate_exponent;
    static int64 exponent_value;
    static int32 return_value;
    
    return_value=1;//overflow (default)
    step=0;
    negate_exponent=0;
    exponent_value=0;
    exponent_digits=0;
    
    //prepare universal number representation
    n_digits=0; n_exp=0; n_neg=0; n_hex=0;
    
    //Out of DATA?
    if (*data_offset>=data_size) return 2;
    
    //read character
    c=data[*data_offset];
    
    // hex/oct/bin
    if (c == 38) {  // "&"
      (*data_offset)++;
      if (*data_offset >= data_size) return 3; // Syntax error (missing H/O/B after &)
      c = data[*data_offset];
      if (c == 44) {
        (*data_offset)++;
        return 3; // Syntax error (missing H/O/B after &)
      }
      if ((c == 72) || (c == 104)) {  // "H" or "h"
      nexthexchr:
        (*data_offset)++;
        if (*data_offset >= data_size) goto gotnumber;
        c = data[*data_offset];
        if (c == 44) {
          (*data_offset)++;
          goto gotnumber;
        }
        if (((c >= 48) && (c <= 57)) || ((c >= 65) && (c <= 70)) ||
            ((c >= 97) && (c <= 102))) {  // 0-9 or A-F or a-f
          if (step == 5) return 3;        // Syntax error (digits after type suffix)
          if (n_digits == 256) return 1;  // Overflow
          n_digit[n_digits] = c;
          n_digits++;
          n_hex = 1;
          goto nexthexchr;
        }
        if ((c == 33) || (c == 35) || (c == 36) ||
            (c == 37) || (c == 38) || (c == 96) || (c == 126)) { // type suffix
            if (step <= 5) {step = 5; goto nexthexchr;}
        }
        return 3;  // Syntax error (invalid HEX char)
      }
      if ((c == 79) || (c == 111)) {  // "O" or "o"
      nexthexchr2:
        (*data_offset)++;
        if (*data_offset >= data_size) goto gotnumber;
        c = data[*data_offset];
        if (c == 44) {
          (*data_offset)++;
          goto gotnumber;
        }
        if ((c >= 48) && (c <= 55)) {     // 0-7
          if (step == 5) return 3;        // Syntax error (digits after type suffix)
          if (n_digits == 256) return 1;  // Overflow
          n_digit[n_digits] = c;
          n_digits++;
          n_hex = 2;
          goto nexthexchr2;
        }
        if ((c == 33) || (c == 35) || (c == 36) ||
            (c == 37) || (c == 38) || (c == 96) || (c == 126)) { // type suffix
            if (step <= 5) {step = 5; goto nexthexchr2;}
        }
        return 3;  // Syntax error (invalid OCT char)
      }
      if ((c == 66) || (c == 98)) {  // "B" or "b"
      nexthexchr3:
        (*data_offset)++;
        if (*data_offset >= data_size) goto gotnumber;
        c = data[*data_offset];
        if (c == 44) {
          (*data_offset)++;
          goto gotnumber;
        }
        if ((c >= 48) && (c <= 49)) {     // 0-1
          if (step == 5) return 3;        // Syntax error (digits after type suffix)
          if (n_digits == 256) return 1;  // Overflow
          n_digit[n_digits] = c;
          n_digits++;
          n_hex = 3;
          goto nexthexchr3;
        }
        if ((c == 33) || (c == 35) || (c == 36) ||
            (c == 37) || (c == 38) || (c == 96) || (c == 126)) { // type suffix
            if (step <= 5) {step = 5; goto nexthexchr3;}
        }
        return 3;  // Syntax error (invalid BIN char)
      }
      return 3;  // Syntax error (missing H/O/B after &)
    }  // "&"
    
    readnextchr:
    if (c==44){(*data_offset)++; goto gotnumber;}
    
    if (c==45){//-
        if (step==0){n_neg=1; step=1; goto nextchr;}//sign before integer part
        if (step==3){negate_exponent=1; step=4; goto nextchr;}//exponent sign
        return 3;//Syntax error (no - allowed in fraction part of number or after type suffix)
    }
    
    if (c==43){//+
        if (step==0){step=1; goto nextchr;}//sign before integer part
        if (step==3){step=4; goto nextchr;}//exponent sign
        return 3;//Syntax error (no + allowed in fraction part of number or after type suffix)
    }
    
    if ((c>=48)&&(c<=57)){//0-9
        if (step == 5) return 3; // Syntax error (digit after type suffix)
        
        if (step<=1){//before decimal point
            step=1;
            if (n_digits||(c>48)){
                if (n_digits) n_exp++;
                if (n_digits==256) return 1;//Overflow
                n_digit[n_digits]=c;
                n_digits++;
            }
        }
        
        if (step==2){//after decimal point
            if ((n_digits==0)&&(c==48)) n_exp--;
            if ((n_digits)||(c>48)){
                if (n_digits==256) return 1;//Overflow
                n_digit[n_digits]=c;
                n_digits++;
            }
        }
        
        if (step>=3){//exponent
            step=4;
            if ((exponent_digits)||(c>48)){
                if (exponent_digits==18) return 1;//Overflow
                exponent_value*=10;
                exponent_value=exponent_value+(c-48);
                exponent_digits++;
            }
        }
        
        goto nextchr;
    }
    
    if (c==46){//.
        if (step>1) return 3;//Syntax error (multiple . or after type suffix)
        if (n_digits==0) n_exp=-1;
        step=2; goto nextchr;
    }
    
    if ((c==68)||(c==69)||(c==100)||(c==101)){//D,E,d,e
        if (step>2) return 3;//Syntax error (multiple exponents or after type suffix)
        step=3; goto nextchr;
    }
    
    if ((c == 33) || (c == 35) || (c == 36) ||
        (c == 37) || (c == 38) || (c == 96) || (c == 126)) { // type suffix
        if (step <= 5) {step = 5; goto nextchr;}
    }
    return 3;//Syntax error (invalid number char)
    nextchr:
    (*data_offset)++; if (*data_offset>=data_size) goto gotnumber;
    c=data[*data_offset];
    goto readnextchr;
    
    gotnumber:;
    if (negate_exponent) n_exp-=exponent_value; else n_exp+=exponent_value;//complete exponent
    if (n_digits==0) {n_exp=0; n_neg=0;}//clarify number
    return 0;//success
}






















void sub_read_string(uint8 *data,ptrszint *data_offset,ptrszint data_size,qbs *deststr){
    if (new_error) return;
    static qbs *str,*character;
    static int32 c,inspeechmarks;
    str=qbs_new(0,0);
    character=qbs_new(1,0);
    inspeechmarks=0;
    
    if (*data_offset>=data_size){error(4); goto gotstr;}//Out of DATA
    
    c=data[*data_offset];
    nextchr:
    
    if (c==44){//,
        if (inspeechmarks!=1){
            (*data_offset)++;
            goto gotstr;
        }
    }
    if (inspeechmarks==2){error(4); str->len=0; goto gotstr;}//syntax error (expected , after closing " unless at end of data in which " is assumed by QB)
    
    if (c==34){//"
        if (inspeechmarks) {inspeechmarks=2; goto skipchr;}
        if (!str->len){inspeechmarks=1; goto skipchr;}
    }
    
    character->chr[0]=c; qbs_set(str,qbs_add(str,character));
    skipchr:
    
    (*data_offset)++; if (*data_offset>=data_size) goto gotstr;
    c=data[*data_offset];
    goto nextchr;
    
    gotstr:
    qbs_set(deststr,str);
    qbs_free(str);
    qbs_free(character);
    return;
}

long double func_read_float(uint8 *data,ptrszint *data_offset,ptrszint data_size,int32 typ){
    if (new_error) return 0;
    static int32 i;
    static int64 maxval,minval;
    static int64 value;
    static ptrszint old_data_offset;
    old_data_offset=*data_offset;
    i=n_inputnumberfromdata(data,data_offset,data_size);
    
    
    //return values:
    //0=success!
    //1=Overflow
    //2=Out of DATA
    //3=Syntax error
    //note: when read fails the data_offset MUST be restored to its old position
    if (i==1){//Overflow
        goto overflow;
    }
    if (i==2){//Out of DATA
        error(4);
        return 0;
    }
    if (i==3){//Syntax error
        *data_offset=old_data_offset;
        error(2); 
        return 0;
    }
    
    
    
    if (!n_float()) goto overflow; //returns n_float_value
    
    //range test & return value
    if (typ&ISFLOAT){
        if ((typ&511)>=64) return n_float_value;
        if (n_float_value>3.402823466E+38) goto overflow;
        return n_float_value;
        }else{
        if (n_float_value<(-(9.2233720368547758E+18)))goto overflow;//too low to store!
        if (n_float_value>   9.2233720368547758E+18)  goto overflow;//too high to store!
        value=qbr(n_float_value);
        if ((typ&ISUNSIGNED)||n_hex){
            maxval=(((int64)1)<<(typ&511))-1;
            minval=0;
            }else{
            maxval=(((int64)1)<<((typ&511)-1))-1;
            minval=-(((int64)1)<<((typ&511)-1));
        }
        if ((value>maxval)||(value<minval)) goto overflow;
        
        if (((typ&ISUNSIGNED)==0)&&n_hex){//signed hex/oct/bin/...  
            if (  ( ((int64)1) << ((typ&511)-1) )  &value){//if top bit is set, set all bits above it to form a negative value
                value=(maxval^((int64)-1))+value;
            }
        }
        
        return value;
    }
    
    overflow:
    *data_offset=old_data_offset;
    error(6); 
    return 0;
}

int64 func_read_int64(uint8 *data,ptrszint *data_offset,ptrszint data_size){
    if (new_error) return 0;
    static int32 i;
    static ptrszint old_data_offset;
    old_data_offset=*data_offset;
    i=n_inputnumberfromdata(data,data_offset,data_size);
    //return values:
    //0=success!
    //1=Overflow
    //2=Out of DATA
    //3=Syntax error
    //note: when read fails the data_offset MUST be restored to its old position
    if (i==1){//Overflow
        goto overflow;
    }
    if (i==2){//Out of DATA
        error(4);
        return 0;
    }
    if (i==3){//Syntax error
        *data_offset=old_data_offset;
        error(2); 
        return 0;
    }
    if (n_int64()) return n_int64_value;
    overflow:
    *data_offset=old_data_offset;
    error(6); 
    return 0;
}

uint64 func_read_uint64(uint8 *data,ptrszint *data_offset,ptrszint data_size){
    if (new_error) return 0;
    static int32 i;
    static ptrszint old_data_offset;
    old_data_offset=*data_offset;
    i=n_inputnumberfromdata(data,data_offset,data_size);
    //return values:
    //0=success!
    //1=Overflow
    //2=Out of DATA
    //3=Syntax error
    //note: when read fails the data_offset MUST be restored to its old position
    if (i==1){//Overflow
        goto overflow;
    }
    if (i==2){//Out of DATA
        error(4);
        return 0;
    }
    if (i==3){//Syntax error
        *data_offset=old_data_offset;
        error(2); 
        return 0;
    }
    if (n_uint64()) return n_uint64_value;
    overflow:
    *data_offset=old_data_offset;
    error(6); 
    return 0;
}


long double func_file_input_float(int32 fileno,int32 typ){
    if (new_error) return 0;
    static int32 i;
    static int64 maxval,minval;
    static int64 value;
    i=n_inputnumberfromfile(fileno);
    if (i==1){error(6); return 0;}//overflow
    if (i==2){error(62); return 0;}//input past end of file
    if (i==3) return 0;//failed
    if (!n_float()){error(6); return 0;} //returns n_float_value
    //range test & return value
    if (typ&ISFLOAT){
        if ((typ&511)>=64) return n_float_value;
        if (n_float_value>3.402823466E+38){error(6); return 0;}
        return n_float_value;
        }else{
        if (n_float_value<(-(9.2233720368547758E+18))){error(6); return 0;}//too low to store!
        if (n_float_value>   9.2233720368547758E+18)  {error(6); return 0;}//too high to store!
        value=qbr(n_float_value);
        if (typ&ISUNSIGNED){
            maxval=(((int64)1)<<(typ&511))-1;
            minval=0;
            }else{
            maxval=(((int64)1)<<((typ&511)-1))-1;
            minval=-(((int64)1)<<((typ&511)-1));
        }
        if ((value>maxval)||(value<minval)){error(6); return 0;}
        
        if (((typ&ISUNSIGNED)==0)&&n_hex){//signed hex/oct/bin/...  
            if (  ( ((int64)1) << ((typ&511)-1) )  &value){//if top bit is set, set all bits above it to form a negative value
                value=(maxval^((int64)-1))+value;
            }
        }
        
        return value;
    }
}//func_file_input_float
