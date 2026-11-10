// Verbatim legacy code from internal/c/libqb.cpp (commit ce58afd), used by tests/diff/diff_core.cpp.
// Do not edit: it is the reference the Rust port is compared against.

uint64 getubits(uint32 bsize,uint8 *base,ptrszint i){
    int64 bmask;
    bmask=~(-(((int64)1)<<bsize));
    i*=bsize;
    return ((*(uint64*)(base+(i>>3)))>>(i&7))&bmask;
}
int64 getbits(uint32 bsize,uint8 *base,ptrszint i){
    int64 bmask, bval64;
    bmask=~(-(((int64)1)<<bsize));
    i*=bsize;
    bval64=((*(uint64*)(base+(i>>3)))>>(i&7))&bmask;
    if (bval64&(((int64)1)<<(bsize-1))) return bval64|(~bmask);
    return bval64;
}
void setbits(uint32 bsize,uint8 *base,ptrszint i,int64 val){
    int64 bmask;
    uint64 *bptr64;
    bmask=(((uint64)1)<<bsize)-1;
    i*=bsize;
    bptr64=(uint64*)(base+(i>>3));
    *bptr64=(*bptr64&( ( (bmask<<(i&7)) ^-1)  )) | ((val&bmask)<<(i&7));
}


    ptrszint func_lbound(ptrszint *array,int32 index,int32 num_indexes){
        if ((index<1)||(index>num_indexes)||((array[2]&1)==0)){
            error(9); return 0;
        }
        index=num_indexes-index+1;
        return array[4*index];
    }
    
    ptrszint func_ubound(ptrszint *array,int32 index,int32 num_indexes){
        if ((index<1)||(index>num_indexes)||((array[2]&1)==0)){
            error(9); return 0;
        }
        index=num_indexes-index+1;
        return array[4*index]+array[4*index+1]-1;
    }
    
    double func_log(double value){
        if (value<=0){error(5);return 0;}
        return log(value);
    }
    
    //FIX
    double func_fix_double(double value){
        if (value<0) return ceil(value); else return floor(value);
    }
    double func_exp_single(double value){
        if (value<=88.02969){
            return exp(value);
        }
        error(6); return 0;
    }
    double func_sqr(double value){
        if (value<0){error(5); return 0;}
        return sqrt(value);
    }
    
    

    qbs *func__bin(int64 value,int32 neg_bits){
        
        static int32 i,i2,i3,neg;
        static int64 value2;
        static qbs *str;
        
        str=qbs_new(64,1);
        
        //negative?
        if ((value>>63)&1) neg=1; else neg=0;
        
        //calc. most significant bit
        i2=0;
        value2=value;
        if (neg){
            for (i=1;i<=64;i++){
                if (!(value2&1)) i2=i;
                value2>>=1;
            }
            if (i2>=neg_bits){
                //doesn't fit in neg_bits, so expand to next 16/32/64 boundary
                i3=64;
                if (i2<32) i3=32;
                if (i2<16) i3=16;
                i2=i3;
            }else i2=neg_bits;
            }else{
            for (i=1;i<=64;i++){
                if (value2&1) i2=i;
                value2>>=1;
            }
        }
        
        if (!i2){str->chr[0]=48; str->len=1; return str;}//"0"
        
        //calc. number of characters required in i3
        i3=i2; // equal for BIN$ because one bit = one char
        
        //build string
        str->len=i3; i3--;
        for (i=1;i<=i2;i++){
            str->chr[i3--]=(value&1)+48;
            value>>=1;
        }
        
        return str;
        
    }
    
    qbs *func_oct(int64 value,int32 neg_bits){
        
        static int32 i,i2,i3,x,x2,neg;
        static int64 value2;
        static qbs *str;
        
        str=qbs_new(22,1);
        
        //negative?
        if ((value>>63)&1) neg=1; else neg=0;
        
        //calc. most significant bit
        i2=0;
        value2=value;
        if (neg){
            for (i=1;i<=64;i++){
                if (!(value2&1)) i2=i;
                value2>>=1;
            }
            if (i2>=neg_bits){
                //doesn't fit in neg_bits, so expand to next 16/32/64 boundary
                i3=64;
                if (i2<32) i3=32;
                if (i2<16) i3=16;
                i2=i3;
            }else i2=neg_bits;
            }else{
            for (i=1;i<=64;i++){
                if (value2&1) i2=i;
                value2>>=1;
            }
        }
        
        if (!i2){str->chr[0]=48; str->len=1; return str;}//"0"
        
        //calc. number of characters required in i3
        i3=i2/3; if ((i3*3)!=i2) i3++;
        
        //build string
        str->len=i3;
        i3--;
        x=0; x2=0;
        for (i=1;i<=i2;i++){
            if (value&1) x2|=(1<<x);
            value>>=1;
            x++;
            if (x==3){str->chr[i3--]=x2+48; x2=0; x=0;}
        }
        if (x) str->chr[i3]=x2+48;
        
        return str;
        
    }
    
    qbs *func_hex(int64 value,int32 neg_size){
        //note: negative int64 values can be treated as positive uint64 values (and vise versa)
        
        static int32 i,i2,i3,x,neg;
        static int64 value2;
        static qbs *str;
        
        str=qbs_new(16,1);
        
        value2=value;
        i2=0; i3=0;
        for (i=1;i<=16;i++){
            if (value2&15) i2=i;//most significant digit of positive value
            if ((value2&15)!=15){
                i3=i;//most significant digit of negative value
                if ((((value2&8)==0)&&(i!=16))) i3++;//for a negative number to fit into 4/8 characters, its top bit must be on
            }
            x=value2&15; if (x>9) x+=55; else x+=48; str->chr[16-i]=x;
            value2>>=4;
        }
        if (!i2){str->chr[0]=48; str->len=1; return str;}//"0"
        
        //negative?
        if ((value>>63)&1) neg=1; else neg=0;
        
        //change i2 from sig-digits to string-output-digits
        if (neg){
            if (i3<=neg_size){
                i2=neg_size;//extend to minimum character size
                }else{
                //didn't fit in recommended size, expand to either 4, 8 or 16 appropriately
                i2=16;
                if (i3<=8) i2=8;
                if (i3<=4) i2=4;
            }
        }//neg
        
        //adjust string to the left to remove unnecessary characters
        if (i2!=16){
            memmove(str->chr,str->chr+(16-i2),i2);
            str->len=i2;
        }
        
        return str;
        
    }
    
