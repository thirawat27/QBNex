// Verbatim copy of legacy func_val from internal/c/libqb.cpp (commit 545b859, lines 13634-13852).
// Test-only reference oracle for diff_val.cpp; production uses crates/qb-runtime/src/val.rs.
long double func_val(qbs *s){
    char c;
    static int32 i,i2,step,num_significant_digits,most_significant_digit_position;
    static int32 num_exponent_digits;
    static int32 negate,negate_exponent;
    static uint8 significant_digits[256];
    static int64 exponent_value;
    static uint8 built_number[256];
    static long double return_value;
    static int64 value;
    static int64 hex_value;
    static int32 hex_digits;
    if (!s->len) return 0;
    value=0;
    negate_exponent=0;
    num_exponent_digits=0;
    num_significant_digits=0;
    most_significant_digit_position=0;
    step=0;
    exponent_value=0;
    negate=0;
    
    i=0;
    for (i=0;i<s->len;i++){
        c=(char)s->chr[i];
        switch (c) {
            case ' ':
            case '\t':
            goto whitespace;
            break;
            
            case '&':
            if (step==0) goto hex;
            goto finish;
            break;
            
            case '-':
            if (step==0) {
                negate = 1;
                step = 1;
                goto checked;
            }
            else if (step==3) {
                negate_exponent = 1;
                step = 4;
                goto checked;
            }
            goto finish;
            break;
            
            case '+':
            if (step==0 || step==3) {
                step++;
                goto checked;
            }
            goto finish;
            break;
            
            case '0':
            case '1':
            case '2':
            case '3':
            case '4':
            case '5':
            case '6':
            case '7':
            case '8':
            case '9':
            if (step<=1){//before decimal point
                step=1;
                if ((num_significant_digits)||(c>48)){
                    most_significant_digit_position++;
                    significant_digits[num_significant_digits]=c;
                    num_significant_digits++;
                    value=value*10+c-48;
                }
            }      
            else if (step==2){//after decimal point
                if ((num_significant_digits==0)&&(c==48)) most_significant_digit_position--;
                if ((num_significant_digits)||(c>48)){
                    significant_digits[num_significant_digits]=c;
                    num_significant_digits++;
                }
            }
            
            else if (step>=3){//exponent
                step=4;
                if ((num_exponent_digits)||(c>48)){
                    if (num_exponent_digits>=18) goto finish;
                    exponent_value*=10; exponent_value=exponent_value+c-48;//precalculate
                    num_exponent_digits++;
                }
            }
            goto checked;
            break;
            
            case '.':
            if (step>1) goto finish;
            step=2; goto checked;
            break;
            
            case 'D':
            case 'E':
            case 'd':
            case 'e':
            if (step>2) goto finish;
            step=3; goto checked;
            break;
            
            default:
            goto finish;//invalid character
            break;
        }
        
        checked:
        whitespace:;
    }
    finish:;
    
    //Check for all-zero mantissa
    if (num_significant_digits==0) return 0;
    
    //If no exponent (or E0) and no decimal part and no chance of overflowing value, return straight away
    if (exponent_value==0 && num_significant_digits==most_significant_digit_position && num_significant_digits < 19){
        return negate ? -value : value;
    }
    
    //normalise number (change 123.456E2 to 1.23456E4, or 123.456 to 1.23456E2)
    exponent_value=exponent_value+most_significant_digit_position-1;
    
    if (negate_exponent) exponent_value=-exponent_value;
    i=0;
    //we are now building a floating point number in ascii characters
    if (negate) {built_number[i]=45; i++;}//-
    if (num_significant_digits){
        //build nomalised mantissa 
        for (i2=0;i2<num_significant_digits;i2++){
            if (i2==1){
                built_number[i]=46;
                i++;
            }
            built_number[i]=significant_digits[i2]; i++;
        }
        built_number[i]=69; i++;//E
        //add exponent
        #ifdef QBNex_WINDOWS
            i2=sprintf((char*)&built_number[i],"%I64i",exponent_value);
            #else
            i2=sprintf((char*)&built_number[i],"%lli",exponent_value);
        #endif
        i=i+i2;
        }else{
        built_number[i]=48; i++;//0
    }
    built_number[i]=0;//NULL terminate
    
    #ifdef QBNex_MINGW
        __mingw_sscanf((char*)&built_number[0],"%Lf",&return_value);
        #else 
        sscanf((char*)&built_number[0],"%Lf",&return_value);
    #endif
    return return_value;
    
    hex://hex/oct
    if (i>=(s->len-2)) return 0;
    c=s->chr[i+1];
    if ((c==79)||(c==111)){//"O"or"o"
        hex_digits=0;
        hex_value=0;
        for (i=i+2;i<s->len;i++){
            c=s->chr[i];
            if ((c>=48)&&(c<=55)){//0-7
                c-=48;
                hex_value<<=3;
                hex_value|=c;
                if (hex_digits||c) hex_digits++; 
                if (hex_digits>=22){
                    if ((hex_digits>22)||(s->chr[i-21]>49)){error(6); return 0;}
                }
            }else break;
        }//i
        return hex_value;
    }
    if ((c==66)||(c==98)){//"B"or"b"
        hex_digits=0;
        hex_value=0;
        for (i=i+2;i<s->len;i++){
            c=s->chr[i];
            if ((c>47)&&(c<50)){//0-1
                c-=48;
                hex_value<<=1;
                hex_value|=c;
                if (hex_digits||c) hex_digits++; 
                if (hex_digits>64){error(6); return 0;}
            }else 
            break;
        }//i
        return hex_value;
    }
    if ((c==72)||(c==104)){//"H"or"h"
        hex_digits=0;
        hex_value=0;
        for (i=i+2;i<s->len;i++){
            c=s->chr[i];
            if ( ((c>=48)&&(c<=57)) || ((c>=65)&&(c<=70)) || ((c>=97)&&(c<=102)) ){//0-9 or A-F or a-f
                if ((c>=48)&&(c<=57)) c-=48;
                if ((c>=65)&&(c<=70)) c-=55;
                
                if ((c>=97)&&(c<=102)) c-=87;
                hex_value<<=4;
                hex_value|=c;
                if (hex_digits||c) hex_digits++;
                if (hex_digits>16) {error(6); return 0;}
            }else break;
        }//i
        return hex_value;
    }
    return 0;//& followied by unknown
}
