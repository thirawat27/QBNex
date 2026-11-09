// Verbatim from internal/c/libqb.cpp (R12a): hexoct2uint64, qbs_input (INPUT / LINE INPUT).
//input helper functions:
uint64 hexoct2uint64_value;
int32 hexoct2uint64(qbs *h) {
  // returns 0 = failed
  //         1 = HEX value (default if unspecified)
  //         2 = OCT value
  //         3 = BIN value
  static int32 i, i2;
  static uint64 result;
  result = 0;
  static int32 type;
  type = 0;
  hexoct2uint64_value = 0;
  if (!h->len) return 1;
  if (h->chr[0] != 38) return 0;  // not "&"
  if (h->len == 1) return 1;      // "&" received, but awaiting further input
  i = h->chr[1];
  if ((i == 72) || (i == 104)) type = 1;  // "H" or "h"
  if ((i == 79) || (i == 111)) type = 2;  // "O" or "o"
  if ((i == 66) || (i ==  98)) type = 3;  // "B" or "b"
  if (!type) return 0;
  if (h->len == 2) return type;

  // unsigned _int64 max = 18446744073709551615   (decimal, 20 chars)
  //                     = 1111111 etc., max. 64x (binary, 64 chars)
  //                     = 1777777777777777777777 (octal, 22 chars)
  //                     = FFFFFFFFFFFFFFFF       (hex, 16 chars)

  if (type == 1) {
    if (h->len > 18) return 0;  // larger than int64
    for (i = 2; i < h->len; i++) {
      result <<= 4;
      i2 = h->chr[i];
      //           0      -      9       /       A      -      F       /       a      -      f
      if (((i2 >= 48) && (i2 <= 57)) || ((i2 >= 65) && (i2 <= 70)) || ((i2 >= 97) && (i2 <= 102))) {
        if (i2 >= 97) i2 -= 32;
        if (i2 >= 65) i2 -= 7;
        i2 -= 48;
        // i2 is now a values between 0 and 15
        result += i2;
      } else
        return 0;  // invalid character
    }  // i
    hexoct2uint64_value = result;
    return 1;
  }  // type == 1

  if (type == 2) {
    if (h->len > 24) return 0;  // larger than int64
    if (h->len == 24) {
      if ((h->chr[2] != 48) && (h->chr[2] != 49))
        return 0;  // larger than int64
    }
    for (i = 2; i < h->len; i++) {
      result <<= 3;
      i2 = h->chr[i];
      if ((i2 >= 48) && (i2 <= 55)) {  // 0-7
        i2 -= 48;
        result += i2;
      } else
        return 0;  // invalid character
    }  // i
    hexoct2uint64_value = result;
    return 2;
  }  // type == 2

  if (type == 3) {
    if (h->len > 66) return 0;  // larger than int64
    for (i = 2; i < h->len; i++) {
      result <<= 1;
      i2 = h->chr[i];
      if ((i2 >= 48) && (i2 <= 49)) {  // 0-1
        i2 -= 48;
        result += i2;
      } else
        return 0;  // invalid character
    }  // i
    hexoct2uint64_value = result;
    return 3;
  }  // type == 3
}


//input method (complex, calls other qbs functions)
const char *uint64_max[] =    {"18446744073709551615"};
const char *int64_max[] =     {"9223372036854775807"};
const char *int64_max_neg[] = {"9223372036854775808"};
const char *single_max[] = {"3402823"};
const char *single_max_neg[] = {"1401298"};
const char *double_max[] = {"17976931"};
const char *double_max_neg[] = {"4940656"};
uint8 significant_digits[1024];
int32 num_significant_digits;

extern void *qbs_input_variableoffsets[257];
extern int32 qbs_input_variabletypes[257];
qbs *qbs_input_arguements[257];
int32 cursor_show_last;

void qbs_input(int32 numvariables,uint8 newline){
    if (new_error) return;
    int32 i,i2,i3,i4,i5,i6,chr;
    
    static int32 autodisplay_backup;
    autodisplay_backup=autodisplay;
    autodisplay=1;
    
    static int32 source_backup;
    source_backup=func__source();
    sub__source(func__dest());
    
    //duplicate dest image so any changes can be reverted
    static int32 dest_image,dest_image_temp,dest_image_holding_cursor;
    dest_image=func__copyimage(func__dest(),NULL,NULL);
    if (dest_image==-1) error(516);//out of memory
    dest_image_temp=func__copyimage(func__dest(),NULL,NULL);
    if (dest_image_temp==-1) error(517);//out of memory
    static int32 dest_image_cursor_x,dest_image_cursor_y;
    dest_image_cursor_x=write_page->cursor_x;
    dest_image_cursor_y=write_page->cursor_y;
    dest_image_holding_cursor=write_page->holding_cursor;
    
    uint32 qbs_tmp_base=qbs_tmp_list_nexti;
    
    static int32 lineinput;
    lineinput=0;
    if (qbs_input_variabletypes[1]&ISSTRING){
        if (qbs_input_variabletypes[1]&512){
            qbs_input_variabletypes[1]=-512;
            lineinput=1;
        }}
        
        cursor_show_last=write_page->cursor_show;
        write_page->cursor_show=1;
        
        int32 addspaces;
        addspaces=0;
        qbs* inpstr=qbs_new(0,0);//not temp so must be freed
        qbs* inpstr2=qbs_new(0,0);//not temp so must be freed
        qbs* key=qbs_new(0,0);//not temp so must be freed
        qbs* c=qbs_new(1,0);//not temp so must be freed
        
        for (i=1;i<=numvariables;i++) qbs_input_arguements[i]=qbs_new(0,0);
        
        //init all passed variables to 0 or ""
        for (i=1;i<=numvariables;i++){
            
            if (qbs_input_variabletypes[i]&ISSTRING){//STRING
                if (((qbs*)qbs_input_variableoffsets[i])->fixed){
                    memset(((qbs*)qbs_input_variableoffsets[i])->chr,32,((qbs*)qbs_input_variableoffsets[i])->len);
                    }else{
                    ((qbs*)qbs_input_variableoffsets[i])->len=0;
                }
            }
            
            if ((qbs_input_variabletypes[i]&ISOFFSETINBITS)==0){//reg. numeric variable
                memset(qbs_input_variableoffsets[i],0,(qbs_input_variabletypes[i]&511)>>3);
            }
            
            //bit referenced?
            
        }//i
        
        
        
        
        qbs_input_next:
        
        int32 argn,firstchr,toomany;
        toomany=0;
        argn=1;
        i=0;
        i2=0;
        qbs_input_arguements[1]->len=0;
        firstchr=1;
        qbs_input_sep_arg:
        
        if (i<inpstr->len){
            
            if (inpstr->chr[i]==44){//","
                if (i2!=1){//not in the middle of a string
                    if (!lineinput){
                        i2=0;
                        argn=argn+1;
                        if (argn>numvariables){toomany=1; goto qbs_input_sep_arg_done;}
                        qbs_input_arguements[argn]->len=0;
                        firstchr=1;
                        goto qbs_input_next_arg;
                    }
                }
            }
            
            if (inpstr->chr[i]==34){//"
                if (firstchr){
                    if (!lineinput){
                        i2=1;//requires closure
                        firstchr=0;
                        goto qbs_input_next_arg;
                    }
                }
                if (i2==1){
                    i2=2;
                    goto qbs_input_next_arg;
                }
            }
            
            if (i2==2){
                goto backspace;//INVALID! Cannot have any characters after a closed "..."
            }
            
            c->chr[0]=inpstr->chr[i];
            qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
            
            firstchr=0;
            qbs_input_next_arg:;
            i++;
            goto qbs_input_sep_arg;
        }
        qbs_input_sep_arg_done:
        if (toomany) goto backspace;
        
        //validate current arguements
        //ASSUME LEADING & TRALING SPACES REMOVED!
        uint8 valid;
        uint8 neg;
        int32 completewith;
        int32 l;
        uint8 *cp,*cp2;
        uint64 max,max_neg,multiple,value;
        uint64 hexvalue;
        
        completewith=-1;
        valid=1;
        l=qbs_input_arguements[argn]->len;
        cp=qbs_input_arguements[argn]->chr;
        neg=0;
        
        if ((qbs_input_variabletypes[argn]&ISSTRING)==0){
            if ((qbs_input_variabletypes[argn]&ISFLOAT)==0){
                if ((qbs_input_variabletypes[argn]&511)<=32){//cannot handle INTEGER64 variables using this method!
                    int64 finalvalue;
                    //it's an integer variable!
                    finalvalue=0;
                    if (l==0){completewith=48; goto typechecked_integer;}
                    //calculate max & max_neg (i4 used to store number of bits)
                    i4=qbs_input_variabletypes[argn]&511;
                    max=1;
                    max<<=i4;
                    max--;
                    
                    //check for hex/oct/bin
                    if (i3=hexoct2uint64(qbs_input_arguements[argn])){
                        hexvalue=hexoct2uint64_value;
                        if (hexvalue>max){valid=0; goto typechecked;}
                        //i. check max num of "digits" required to represent a value, if more exist cull excess
                        //ii. set completewith value (if necessary)
                        if (i3==1){
                            value=max;
                            i=0;
                            for (i2=1;i2<=16;i2++){
                                if (value&0xF) i=i2;
                                value>>=4;
                            }
                            if (l>(2+i)){valid=0; goto typechecked;}
                            if (l==1) completewith=72;//"H"
                            if (l==2) completewith=48;//"0"
                        }
                        if (i3==2){
                            value=max;
                            i=0;
                            for (i2=1;i2<=22;i2++){
                                if (value&0x7) i=i2;
                                value>>=3;
                            }
                            if (l>(2+i)){valid=0; goto typechecked;}
                            if (l==1) completewith=79;//"O"
                            if (l==2) completewith=48;//"0"
                        }
                        if (i3==3){
                            value=max;
                            i=0;
                            for (i2=1;i2<=64;i2++){
                                if (value&0x1) i=i2;
                                value>>=1;
                            }
                            if (l>(2+i)){valid=0; goto typechecked;}
                            if (l==1) completewith=66;//"B"
                            if (l==2) completewith=48;//"0"
                        }
                        finalvalue=hexvalue;
                        goto typechecked_integer;
                    }
                    
                    //max currently contains the largest UNSIGNED value possible, adjust as necessary
                    if (qbs_input_variabletypes[argn]&ISUNSIGNED){ 
                        max_neg=0;
                        }else{
                        max>>=1;
                        max_neg=max+1;
                    }
                    //check for - sign
                    i2=0;
                    if ((qbs_input_variabletypes[argn]&ISUNSIGNED)==0){ 
                        if (cp[i2]==45){//"-"
                            if (l==1) {completewith=48; goto typechecked_integer;}
                            i2++; neg=1;
                        }
                    }
                    //after a leading 0 no other digits are possible, return an error if this is the case
                    if (cp[i2]==48){
                        if (l>(i2+1)){valid=0; goto typechecked;}
                    }
                    //scan the "number"...
                    multiple=1;
                    value=0;
                    for (i=l-1;i>=i2;i--){
                        i3=cp[i]-48;
                        if ((i3>=0)&&(i3<=9)){
                            value+=multiple*i3;
                            if (qbs_input_variabletypes[argn]&ISUNSIGNED){ 
                                if (value>max){valid=0; goto typechecked;}
                                }else{
                                if (neg){
                                    if (value>max_neg){valid=0; goto typechecked;}
                                    }else{
                                    if (value>max){valid=0; goto typechecked;}
                                }
                            }
                        }else{valid=0; goto typechecked;}
                        multiple*=10;
                    }//next i
                    if (neg) finalvalue=-value; else finalvalue=value;
                    typechecked_integer:
                    //set variable to finalvalue
                    if ((qbs_input_variabletypes[argn]&ISOFFSETINBITS)==0){//reg. numeric variable
                        memcpy(qbs_input_variableoffsets[argn],&finalvalue,(qbs_input_variabletypes[argn]&511)>>3);
                    }
                    goto typechecked;
                }
            }
        }
        
        if (qbs_input_variabletypes[argn]&ISSTRING){
            if (((qbs*)qbs_input_variableoffsets[argn])->fixed){
                if (l>((qbs*)qbs_input_variableoffsets[argn])->len) {valid=0; goto typechecked;}
            }
            qbs_set((qbs*)qbs_input_variableoffsets[argn],qbs_input_arguements[argn]);
            goto typechecked;
        }
        
        //INTEGER64 type
        //int64 range:          \969223372036854775808 to  9223372036854775807
        //uint64 range: 0                    to 18446744073709551615
        if ((qbs_input_variabletypes[argn]&ISSTRING)==0){
            if ((qbs_input_variabletypes[argn]&ISFLOAT)==0){
                if ((qbs_input_variabletypes[argn]&511)==64){
                    if (l==0){completewith=48; *(int64*)qbs_input_variableoffsets[argn]=0; goto typechecked;}
                    
                    //check for hex/oct/bin
                    if (i3=hexoct2uint64(qbs_input_arguements[argn])){
                        hexvalue=hexoct2uint64_value;
                        if (hexvalue>max){valid=0; goto typechecked;}
                        //set completewith value (if necessary)
                        if (i3==1) if (l==1) completewith=72;//"H"
                        if (i3==2) if (l==1) completewith=79;//"O"
                        if (i3==3) if (l==1) completewith=66;//"B"
                        if (l==2) completewith=48;//"0"
                        *(uint64*)qbs_input_variableoffsets[argn]=hexvalue;
                        goto typechecked;
                    }
                    
                    //check for - sign
                    i2=0;
                    if ((qbs_input_variabletypes[argn]&ISUNSIGNED)==0){ 
                        if (cp[i2]==45){//"-"
                            if (l==1) {completewith=48; *(int64*)qbs_input_variableoffsets[argn]=0; goto typechecked;}
                            i2++; neg=1;
                        }
                    }
                    //after a leading 0 no other digits are possible, return an error if this is the case
                    if (cp[i2]==48){
                        if (l>(i2+1)){valid=0; goto typechecked;}
                    }
                    //count how many digits are in the number
                    i4=0;
                    for (i=l-1;i>=i2;i--){
                        i3=cp[i]-48;
                        if ((i3<0)||(i3>9)) {valid=0; goto typechecked;}
                        i4++;
                    }//i
                    if (qbs_input_variabletypes[argn]&ISUNSIGNED){
                        if (i4<20) goto typechecked_int64;
                        if (i4>20) {valid=0; goto typechecked;}
                        
                        cp2=(uint8*)uint64_max[0];
                        }else{
                        if (i4<19) goto typechecked_int64;
                        if (i4>19) {valid=0; goto typechecked;}
                        if (neg) cp2=(uint8*)int64_max_neg[0]; else cp2=(uint8*)int64_max[0];
                    }
                    //number of digits valid, but exact value requires checking
                    cp=qbs_input_arguements[argn]->chr;
                    for (i=0;i<i4;i++){
                        if (cp[i+i2]<cp2[i]) goto typechecked_int64;
                        if (cp[i+i2]>cp2[i]) {valid=0; goto typechecked;}
                    }
                    typechecked_int64:
                    //add character 0 to end to make it a null terminated string
                    c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                    if (qbs_input_variabletypes[argn]&ISUNSIGNED){
                        #ifdef QBNex_WINDOWS
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%I64u",(uint64*)qbs_input_variableoffsets[argn]);
                            #else
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%llu",(uint64*)qbs_input_variableoffsets[argn]);
                        #endif
                        }else{
                        #ifdef QBNex_WINDOWS
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%I64i",(int64*)qbs_input_variableoffsets[argn]);
                            #else
                            sscanf((char*)qbs_input_arguements[argn]->chr,"%lli",(int64*)qbs_input_variableoffsets[argn]);
                        #endif
                    }
                    goto typechecked;
                }
            }
        }
        
        //check ISFLOAT type?
        //[-]9999[.]9999[E/D][+/-]99999
        if (qbs_input_variabletypes[argn]&ISFLOAT){
            static int32 digits_before_point;
            static int32 digits_after_point;
            static int32 zeros_after_point;
            static int32 neg_power;
            digits_before_point=0;
            digits_after_point=0;
            neg_power=0;
            value=0;
            zeros_after_point=0;
            num_significant_digits=0;
            
            //set variable to 0
            if ((qbs_input_variabletypes[argn]&511)==32) *(float*)qbs_input_variableoffsets[argn]=0;
            if ((qbs_input_variabletypes[argn]&511)==64) *(double*)qbs_input_variableoffsets[argn]=0;
            if ((qbs_input_variabletypes[argn]&511)==256) *(long double*)qbs_input_variableoffsets[argn]=0;
            
            //begin with a generic assessment, regardless of whether it is single, double or float
            if (l==0){completewith=48; goto typechecked;}
            
            //check for hex/oct/bin
            if (i3=hexoct2uint64(qbs_input_arguements[argn])){
                hexvalue=hexoct2uint64_value;
                //set completewith value (if necessary)
                if (i3==1) if (l==1) completewith=72;//"H"
                if (i3==2) if (l==1) completewith=79;//"O"
                if (i3==3) if (l==1) completewith=66;//"B"
                if (l==2) completewith=48;//"0"
                //nb. because VC6 didn't support...
                //error C2520: conversion from uint64 to double not implemented, use signed int64
                //I've implemented a work-around so correct values will be returned
                static int64 transfer;
                transfer=0x7FFFFFFF;
                transfer<<=32;
                transfer|=0xFFFFFFFF;
                while(hexvalue>transfer){
                    hexvalue-=transfer;
                    if ((qbs_input_variabletypes[argn]&511)==32) *(float*)qbs_input_variableoffsets[argn]+=transfer;
                    if ((qbs_input_variabletypes[argn]&511)==64) *(double*)qbs_input_variableoffsets[argn]+=transfer;
                    if ((qbs_input_variabletypes[argn]&511)==256) *(long double*)qbs_input_variableoffsets[argn]+=transfer;
                }
                transfer=hexvalue;
                if ((qbs_input_variabletypes[argn]&511)==32) *(float*)qbs_input_variableoffsets[argn]+=transfer;
                if ((qbs_input_variabletypes[argn]&511)==64) *(double*)qbs_input_variableoffsets[argn]+=transfer;
                if ((qbs_input_variabletypes[argn]&511)==256) *(long double*)qbs_input_variableoffsets[argn]+=transfer;
                goto typechecked;
            }
            
            //check for - sign
            i2=0;
            if (cp[i2]==45){//"-"
                if (l==1) {completewith=48; goto typechecked;}
                i2++; neg=1;
            }
            //if it starts with 0, it may only have one leading 0
            if (cp[i2]==48){
                if (l>(i2+1)){
                    i2++;
                    if (cp[i2]==46) goto decimal_point;
                    valid=0; goto typechecked;//expected a decimal point
                    //nb. of course, user could have typed D or E BUT there is no point
                    //    calculating 0 to the power of anything!
                }else goto typechecked;//validate, as no other data is required
            }
            //scan digits before decimal place
            for (i=i2;i<l;i++){
                i3=cp[i];
                if ((i3==68)||(i3==(68+32))||(i3==69)||(i3==(69+32))){//d,D,e,E?
                    if (i==i2){valid=0; goto typechecked;}//cannot begin with d,D,e,E!
                    i2=i;
                    goto exponent;
                }
                if (i3==46){i2=i; goto decimal_point;}//nb. it can begin with a decimal point!
                i3-=48;
                if ((i3<0)||(i3>9)){valid=0; goto typechecked;}
                digits_before_point++;
                //nb. because leading 0 is handled differently, all digits are significant
                significant_digits[num_significant_digits]=i3+48; num_significant_digits++;
            }
            goto assess_float;
            ////////////////////////////////
            decimal_point:;
            i4=1;
            if (i2==(l-1)) {completewith=48; goto assess_float;}
            i2++;
            for (i=i2;i<l;i++){
                i3=cp[i];
                if ((i3==68)||(i3==(68+32))||(i3==69)||(i3==(69+32))){//d,D,e,E?
                    if (num_significant_digits){
                        if (i==i2){valid=0; goto typechecked;}//cannot begin with d,D,e,E just after a decimal point!
                        i2=i;
                        goto exponent;
                    }
                }
                i3-=48;
                if ((i3<0)||(i3>9)){valid=0; goto typechecked;}
                if (i3) i4=0;
                if (i4) zeros_after_point++;
                digits_after_point++;
                if ((num_significant_digits)||i3){
                    significant_digits[num_significant_digits]=i3+48; num_significant_digits++;
                }
            }//i
            goto assess_float;
            ////////////////////////////////
            exponent:;
            
            //ban d/D for SINGLE precision input
            if ((qbs_input_variabletypes[argn]&511)==32){//SINGLE
                i3=cp[i2];
                if ((i3==68)||(i3==(68+32))){//d/D
                    valid=0; goto typechecked;
                }
            }
            //correct "D" notation for c++ scanf
            i3=cp[i2];
            if ((i3==68)||(i3==(68+32))){//d/D
                cp[i2]=69;//"E"
            }
            if (i2==(l-1)) {completewith=48; goto assess_float;}
            i2++;
            //check for optional + or -
            i3=cp[i2];
            if (i3==45){//"-"
                if (i2==(l-1)) {completewith=48; goto assess_float;}
                neg_power=1;
                i2++;
            }
            if (i3==43){//"+"
                if (i2==(l-1)) {completewith=48; goto assess_float;}
                i2++;
            }
            //nothing valid after a leading 0
            if (cp[i2]==48){//0
                if (l>(i2+1)) {valid=0; goto typechecked;}
            }
            multiple=1;
            value=0;
            for (i=l-1;i>=i2;i--){
                i3=cp[i]-48;
                if ((i3>=0)&&(i3<=9)){
                    
                    value+=multiple*i3;
                    }else{
                    valid=0; goto typechecked;
                }
                multiple*=10;
            }//i
            //////////////////////////
            assess_float:;
            //nb. 0.???? means digits_before_point==0
            
            if ((qbs_input_variabletypes[argn]&511)==32){//SINGLE
                //QB:           \B13.402823    E+38 to \B11.401298    E-45
                //WIKIPEDIA:    \B13.4028234   E+38 to ?
                //OTHER SOURCE: \B13.402823466 E+38 to \B11.175494351 E-38
                if (neg_power) value=-value;
                //special case->single 0 after point
                if ((zeros_after_point==1)&&(digits_after_point==1)){
                    digits_after_point=0;
                    zeros_after_point=0;
                }
                //upper overflow check
                //i. check that value doesn't consist solely of 0's
                if (zeros_after_point>43){valid=0; goto typechecked;}//cannot go any further without reversal by exponent
                if ((digits_before_point==0)&&(digits_after_point==zeros_after_point)) goto nooverflow_float;
                //ii. calculate the position of the first WHOLE digit (in i)
                i=digits_before_point;
                if (!i) i=-zeros_after_point;
                /*EXAMPLES:
                    1.0         i=1
                    12.0        i=2
                    0.1         i=0
                    0.01        i=-1
                */
                i=i+value;//apply exponent
                if (i>39){valid=0; goto typechecked;}
                //nb. the above blocks the ability to type a long-int32 number and use a neg exponent
                //    to validate it
                //********IMPORTANT: if i==39 then the first 7 digits MUST be scanned!!!
                if (i==39){
                    cp2=(uint8*)single_max[0];
                    i2=num_significant_digits;
                    if (i2>7) i2=7;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]>*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]<*cp2) break;
                        cp2++;
                    }
                }
                //check for pointless levels of precision (eg. 1.21351273512653625116212!)
                if (digits_after_point){
                    if (digits_before_point){
                        if ((digits_after_point+digits_before_point)>8){valid=0; goto typechecked;}
                        }else{
                        if ((digits_after_point-zeros_after_point)>8){valid=0; goto typechecked;}
                    }
                }
                //check for "under-flow"
                if (i<-44){valid=0; goto typechecked;}
                //********IMPORTANT: if i==-44 then the first 7 digits MUST be scanned!!!
                if (i==-44){
                    cp2=(uint8*)single_max_neg[0];
                    i2=num_significant_digits;
                    if (i2>7) i2=7;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]<*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]>*cp2) break;
                        cp2++;
                    }
                }
                nooverflow_float:;
                c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                sscanf((char*)qbs_input_arguements[argn]->chr,"%f",(float*)qbs_input_variableoffsets[argn]);
                goto typechecked;
            }
            
            if ((qbs_input_variabletypes[argn]&511)==64){//DOUBLE
                //QB: Double (15-digit) precision \B11.7976931 D+308 to \B14.940656 D-324
                //WIKIPEDIA:    \B11.7976931348623157 D+308 to ???
                //OTHER SOURCE: \B11.7976931348623157 D+308 to \B12.2250738585072014E-308
                
                
                
                if (neg_power) value=-value;
                //special case->single 0 after point
                if ((zeros_after_point==1)&&(digits_after_point==1)){
                    digits_after_point=0;
                    zeros_after_point=0;
                }
                //upper overflow check
                //i. check that value doesn't consist solely of 0's
                if (zeros_after_point>322){valid=0; goto typechecked;}//cannot go any further without reversal by exponent
                if ((digits_before_point==0)&&(digits_after_point==zeros_after_point)) goto nooverflow_double;
                //ii. calculate the position of the first WHOLE digit (in i)
                i=digits_before_point;
                if (!i) i=-zeros_after_point;
                i=i+value;//apply exponent
                if (i>309){valid=0; goto typechecked;}
                //nb. the above blocks the ability to type a long-int32 number and use a neg exponent
                //    to validate it
                //********IMPORTANT: if i==309 then the first 8 digits MUST be scanned!!!
                if (i==309){
                    cp2=(uint8*)double_max[0];
                    i2=num_significant_digits;
                    if (i2>8) i2=8;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]>*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]<*cp2) break;
                        cp2++;
                    }
                }
                //check for pointless levels of precision (eg. 1.21351273512653625116212!)
                if (digits_after_point){
                    if (digits_before_point){
                        if ((digits_after_point+digits_before_point)>16){valid=0; goto typechecked;}
                        }else{
                        if ((digits_after_point-zeros_after_point)>16){valid=0; goto typechecked;}
                    }
                }
                //check for "under-flow"
                if (i<-323){valid=0; goto typechecked;}
                //********IMPORTANT: if i==-323 then the first 7 digits MUST be scanned!!!
                if (i==-323){
                    cp2=(uint8*)double_max_neg[0];
                    i2=num_significant_digits;
                    if (i2>7) i2=7;
                    for (i3=0;i3<i2;i3++){
                        if (significant_digits[i3]<*cp2){valid=0; goto typechecked;}
                        if (significant_digits[i3]>*cp2) break;
                        cp2++;
                    }
                }
                nooverflow_double:;
                c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                sscanf((char*)qbs_input_arguements[argn]->chr,"%lf",(double*)qbs_input_variableoffsets[argn]);
                goto typechecked;
            }
            
            if ((qbs_input_variabletypes[argn]&511)==256){//FLOAT
                //at present, there is no defined limit for FLOAT type numbers, so no restrictions
                //are applied!
                c->chr[0]=0; qbs_set(qbs_input_arguements[argn],qbs_add(qbs_input_arguements[argn],c));
                
                //sscanf((char*)qbs_input_arguements[argn]->chr,"%lf",(long double*)qbs_input_variableoffsets[argn]);
                static double sscanf_fix;
                sscanf((char*)qbs_input_arguements[argn]->chr,"%lf",&sscanf_fix);
                *(long double*)qbs_input_variableoffsets[argn]=sscanf_fix;
                
            }
            
            
        }//ISFLOAT
        
        //undefined/uncheckable types fall through as valid!
        typechecked:;
        if (!valid) goto backspace;
        
        
        
        qbs_set(inpstr2,inpstr);
        
        
        //input a key
        
        
        
        qbs_input_invalidinput:
        
        static int32 showing_cursor;
        showing_cursor=0;
        
        qbs_input_wait_for_key:
        
        //toggle box cursor
        if (!write_page->text){
            i=1;
            if ((write_page->font>=32)||(write_page->compatible_mode==256)||(write_page->compatible_mode==32)){
                if (GetTicks()&512) i=0;
            }
            if (i!=showing_cursor){
                showing_cursor^=1;
                static int32 x,y,x2,y2,fx,fy,alpha,cw;
                static uint32 c;
                alpha=write_page->alpha_disabled; write_page->alpha_disabled=1;
                fy=fontheight[write_page->font];
                fx=fontwidth[write_page->font]; if (!fx) fx=1;
                cw=fx; if ((write_page->font>=32)||(write_page->compatible_mode==256)||(write_page->compatible_mode==32)) cw=1;
                y2=(write_page->cursor_y-1)*fy;
                for (y=0;y<fy;y++){
                    x2=(write_page->cursor_x-1)*fx;
                    for (x=0;x<cw;x++){
                        pset (x2,y2,point(x2,y2)^write_page->color);
                        x2++;
                    }
                    y2++;
                }
                write_page->alpha_disabled=alpha;
            }
        }//!write_page->text
        
        if (addspaces){
            addspaces--;
            c->chr[0]=32; qbs_set(key,c);
            }else{
            
            if (write_page->console){
                qbs_set(key,qbs_new_txt(""));
                chr=fgetc(stdin);
                if (chr!=EOF){
                    if (chr=='\n') chr=13;
                    qbs_set(key,qbs_new_txt(" "));  
                    key->chr[0]=chr;  
                }else{Sleep(10);}
                }else{
                Sleep(10);
                qbs_set(key,qbs_inkey());
                
            }
            
            qbs_cleanup(qbs_tmp_base,0);
        }
        if (stop_program) return;
        if (key->len!=1) goto qbs_input_wait_for_key;
        
        //remove box cursor
        if (!write_page->text){
            if (showing_cursor){
                showing_cursor=0;
                static int32 x,y,x2,y2,fx,fy,cw,alpha;
                static uint32 c;
                alpha=write_page->alpha_disabled; write_page->alpha_disabled=1;
                fy=fontheight[write_page->font];
                fx=fontwidth[write_page->font]; if (!fx) fx=1;
                cw=fx; if ((write_page->font>=32)||(write_page->compatible_mode==256)||(write_page->compatible_mode==32)) cw=1;
                y2=(write_page->cursor_y-1)*fy;
                for (y=0;y<fy;y++){
                    x2=(write_page->cursor_x-1)*fx;
                    for (x=0;x<cw;x++){
                        pset (x2,y2,point(x2,y2)^write_page->color);
                        x2++;
                    }
                    y2++;
                }
                write_page->alpha_disabled=alpha;
            }
        }//!write_page->text
        
        //input should disallow certain characters
        if (key->chr[0]==7) {qbs_print(key,0); goto qbs_input_next;}//beep!
        if (key->chr[0]==10) goto qbs_input_next;//linefeed
        if (key->chr[0]==9){//tab
            i=8-(inpstr2->len&7);
            addspaces=i;
            goto qbs_input_next;
        }
        //other ASCII chars that cannot be printed
        if (key->chr[0]==11) goto qbs_input_next;
        if (key->chr[0]==12) goto qbs_input_next;
        if (key->chr[0]==28) goto qbs_input_next;
        if (key->chr[0]==29) goto qbs_input_next;
        if (key->chr[0]==30) goto qbs_input_next;
        if (key->chr[0]==31) goto qbs_input_next;
        
        if (key->chr[0]==13){
            //assume input is valid
            
            //autofinish (if necessary)
            
            //assume all parts entered
            
            for (i=1;i<=numvariables;i++){
                qbs_free(qbs_input_arguements[i]);
            }
            
            if (newline){
                c->len=0;
                if (!write_page->console) qbs_print(c,1);
            }
            qbs_free(c);
            qbs_free(inpstr);
            qbs_free(inpstr2);
            qbs_free(key);
            
            write_page->cursor_show=cursor_show_last;
            
            sub__freeimage(dest_image,1); sub__freeimage(dest_image_temp,1);
            
            if (autodisplay_backup==0){
                autodisplay=-1;//toggle request
                while(autodisplay) Sleep(1);
            }
            
            sub__source(source_backup);
            
            return;
        }
        
        if (key->chr[0]==8){//backspace
            backspace:
            if (!inpstr->len) goto qbs_input_invalidinput;
            inpstr->len--;
            i2=func__dest();//backup current dest
            sub_pcopy(dest_image,dest_image_temp);//copy original background to temp
            //write characters to temp
            sub__dest(dest_image_temp);
            write_page->cursor_x=dest_image_cursor_x; write_page->cursor_y=dest_image_cursor_y; write_page->holding_cursor=dest_image_holding_cursor;
            for (i=0;i<inpstr->len;i++){key->chr[0]=inpstr->chr[i]; qbs_print(key,0);}
            sub__dest(i2);
            //copy temp to dest
            sub_pcopy(dest_image_temp,i2);
            //update cursor
            write_page->cursor_x=img[-dest_image_temp].cursor_x; write_page->cursor_y=img[-dest_image_temp].cursor_y;
            goto qbs_input_next;
        }
        
        //affect inpstr2 with key
        qbs_set(inpstr2,qbs_add(inpstr2,key));
        
        //perform actual update
        if (!write_page->console) qbs_print(key,0);
        
        qbs_set(inpstr,inpstr2);
        
        goto qbs_input_next;
        
}//qbs_input
