// VERBATIM copy of the legacy string engine, test-only (differential harness diff_qbs.cpp).
// Source: internal/c/libqb.cpp @ 81931ee lines 6712-7339, 7456-7673, 7741-7819;
//         internal/c/qbx.cpp @ 81931ee lines 808-817 (qbs_cleanup; the mistyped
//         'extern ptrszint *qbs_tmp_list;' on line 807 (debt F2) is left out).
// Do not edit: it is the oracle the Rust port (qbrt/src/qbs.rs) is compared against.
static const uint32 QBS_DESCRIPTOR_BLOCK_SIZE=65536;
static const uint32 QBS_TRACK_LIST_BLOCK_SIZE=65536;
static std::vector<std::unique_ptr<qbs[]>> qbs_malloc_blocks;
qbs *qbs_malloc=NULL;
uint32 qbs_malloc_next=0;//the next idex in qbs_malloc to use
static std::vector<qbs*> qbs_malloc_freed;

static qbs* qbs_allocate_descriptor_block(){
    std::unique_ptr<qbs[]> block(new(std::nothrow) qbs[QBS_DESCRIPTOR_BLOCK_SIZE]());
    if (!block){
        error(508);
        return NULL;
    }
    qbs *block_ptr=block.get();
    qbs_malloc_blocks.push_back(std::move(block));
    return block_ptr;
}

static void qbs_expand_tracking_list(std::vector<ptrszint> &list,uint32 &lasti){
    const size_t new_size=std::max<size_t>(list.size()*2, static_cast<size_t>(QBS_TRACK_LIST_BLOCK_SIZE));
    list.resize(new_size, 0);
    lasti=static_cast<uint32>(new_size-1);
}

/*MLP
    uint32 *dbglist=(uint32*)malloc(4*10000000);
    uint32 dbglisti=0;
    uint32 dbgline=0;
*/

qbs *qbs_new_descriptor(){
    //MLP //qbshlp1++;
    if (!qbs_malloc_freed.empty()){
        /*MLP
            static qbs *s;
            s=(qbs*)memset((void *)qbs_malloc_freed.back(),0,sizeof(qbs));
            s->dbgl=dbgline;
            qbs_malloc_freed.pop_back();
            return s;
        */
        qbs *descriptor=qbs_malloc_freed.back();
        qbs_malloc_freed.pop_back();
        return (qbs*)memset((void *)descriptor,0,sizeof(qbs));
    }
    if (qbs_malloc==NULL || qbs_malloc_next==QBS_DESCRIPTOR_BLOCK_SIZE){
        qbs_malloc=qbs_allocate_descriptor_block();//~1MEG
        qbs_malloc_next=0;
    }
    /*MLP
        dbglist[dbglisti]=(uint32)&qbs_malloc[qbs_malloc_next];
        static qbs* s;
        s=(qbs*)&qbs_malloc[qbs_malloc_next++];
        s->dbgl=dbgline;
        dbglisti++;
        return s;
    */
    return &qbs_malloc[qbs_malloc_next++];
}

void qbs_free_descriptor(qbs *str){
    //MLP //qbshlp1--;
    qbs_malloc_freed.push_back(str);
    return;
}

//Used to track strings in 16bit memory
std::vector<ptrszint> qbs_cmem_list(QBS_TRACK_LIST_BLOCK_SIZE,0);
uint32  qbs_cmem_list_lasti=65535;
uint32  qbs_cmem_list_nexti=0;
//Used to track strings in 32bit memory
std::vector<ptrszint> qbs_list(QBS_TRACK_LIST_BLOCK_SIZE,0);
uint32  qbs_list_lasti=65535;
uint32  qbs_list_nexti=0;
//Used to track temporary strings for later removal when they fall out of scope
//*Some string functions delete a temporary string automatically after they have been
// passed one to save memory. In this case qbstring_templist[?]=0xFFFFFFFF
std::vector<ptrszint> qbs_tmp_list(QBS_TRACK_LIST_BLOCK_SIZE,0);//first index MUST be 0
uint32  qbs_tmp_list_lasti=65535;
extern uint32 qbs_tmp_list_nexti;
//entended string memory

static std::vector<uint8> qbs_data_storage(1048576,0);
uint8 *qbs_data=qbs_data_storage.data();
uint32 qbs_data_size=static_cast<uint32>(qbs_data_storage.size());
uint32 qbs_sp=0;


void field_free(qbs* str);

void qbs_free(qbs *str){
    
    if (str->field) field_free(str);
    
    if (str->tmplisti){
        qbs_tmp_list[str->tmplisti]=-1;
        while (qbs_tmp_list[qbs_tmp_list_nexti-1]==-1){
            qbs_tmp_list_nexti--;
        }
    }
    if (str->fixed||str->readonly){
        qbs_free_descriptor(str);
        return;
    }
    if (str->in_cmem){
        qbs_cmem_list[str->listi]=-1;
        if ((qbs_cmem_list_nexti-1)==str->listi) qbs_cmem_list_nexti--;
        }else{
        qbs_list[str->listi]=-1;
        retry:
        if (qbs_list[qbs_list_nexti-1]==-1){
            qbs_list_nexti--;
            if (qbs_list_nexti) goto retry;
        }
        if (qbs_list_nexti){
            qbs_sp=((qbs*)qbs_list[qbs_list_nexti-1])->chr-qbs_data+((qbs*)qbs_list[qbs_list_nexti-1])->len+32;
            if (qbs_sp>qbs_data_size) qbs_sp=qbs_data_size;//adding 32 could overflow buffer!
            }else{
            qbs_sp=0;
        }
    }
    qbs_free_descriptor(str);
    return;
}

void qbs_cmem_concat_list(){
    uint32 i;
    uint32 d;
    qbs *tqbs;
    d=0;
    for (i=0;i<qbs_cmem_list_nexti;i++){
        if (qbs_cmem_list[i]!=-1){ 
            if (i!=d){  
                tqbs=(qbs*)qbs_cmem_list[i];
                tqbs->listi=d;
                qbs_cmem_list[d]=(ptrszint)tqbs;
            }
            d++;
        }
    }
    qbs_cmem_list_nexti=d;
    //if string listings are taking up more than half of the list array double the list array's size
    if (qbs_cmem_list_nexti>=(qbs_cmem_list_lasti/2)){
        qbs_expand_tracking_list(qbs_cmem_list,qbs_cmem_list_lasti);
    }
    return;
}

void qbs_concat_list(){
    uint32 i;
    uint32 d;
    qbs *tqbs;
    d=0;
    for (i=0;i<qbs_list_nexti;i++){
        if (qbs_list[i]!=-1){
            if (i!=d){
                tqbs=(qbs*)qbs_list[i];
                tqbs->listi=d;
                qbs_list[d]=(ptrszint)tqbs;
            }
            d++;
        }
    }
    qbs_list_nexti=d;
    //if string listings are taking up more than half of the list array double the list array's size
    if (qbs_list_nexti>=(qbs_list_lasti/2)){
        qbs_expand_tracking_list(qbs_list,qbs_list_lasti);
    }
    return;
}

void qbs_tmp_concat_list(){
    if (qbs_tmp_list_nexti>=(qbs_tmp_list_lasti/2)){
        qbs_expand_tracking_list(qbs_tmp_list,qbs_tmp_list_lasti);
    }
    return;
}




void qbs_concat(uint32 bytesrequired){
    //this does not change indexing, only ->chr pointers and the location of their data
    static int32 i;
    static uint8 *dest;
    static qbs *tqbs;
    dest=(uint8*)qbs_data;
    if (qbs_list_nexti){
        qbs_sp=0;
        for (i=0;i<qbs_list_nexti;i++){
            if (qbs_list[i]!=-1){
                tqbs=(qbs*)qbs_list[i];
                if ((tqbs->chr-dest)>32){
                    if (tqbs->len) {memmove(dest,tqbs->chr,tqbs->len);}
                    tqbs->chr=dest;       
                }
                dest=tqbs->chr+tqbs->len;
                qbs_sp=dest-qbs_data;
            }
        }
    }
    
    if (((qbs_sp*2)+(bytesrequired+32))>=qbs_data_size){
        static uint8 *oldbase;
        oldbase=qbs_data;
        qbs_data_size=qbs_data_size*2+bytesrequired;
        qbs_data_storage.resize(qbs_data_size);
        qbs_data=qbs_data_storage.data();
        for (i=0;i<qbs_list_nexti;i++){
            if (qbs_list[i]!=-1){
                tqbs=(qbs*)qbs_list[i];
                tqbs->chr=tqbs->chr-oldbase+qbs_data;
            }
        }
    }
    return;
}

//as the cmem stack has a limit if bytesrequired cannot be met this exits and returns an error
//the cmem stack cannot after all be extended!
//so bytesrequired is only passed to possibly generate an error, or not generate one
void qbs_concat_cmem(uint32 bytesrequired){
    //this does not change indexing, only ->chr pointers and the location of their data
    int32 i;
    uint8 *dest;
    qbs *tqbs;
    dest=(uint8*)dblock;
    qbs_cmem_sp=qbs_cmem_descriptor_space;
    if (qbs_cmem_list_nexti){
        for (i=0;i<qbs_cmem_list_nexti;i++){
            if (qbs_cmem_list[i]!=-1){
                tqbs=(qbs*)qbs_cmem_list[i];
                if (tqbs->chr!=dest){
                    if (tqbs->len) {memmove(dest,tqbs->chr,tqbs->len);}
                    tqbs->chr=dest;
                    //update cmem_descriptor [length][offset]
                    if (tqbs->cmem_descriptor){tqbs->cmem_descriptor[0]=tqbs->len; tqbs->cmem_descriptor[1]=(uint16)(ptrszint)(tqbs->chr-dblock);}
                }
                dest+=tqbs->len;
                qbs_cmem_sp+=tqbs->len;
            }
        }
    }
    if ((qbs_cmem_sp+bytesrequired)>cmem_sp) error(513);
    return;
}

qbs *qbs_new_cmem(int32 size,uint8 tmp){
    if ((qbs_cmem_sp+size)>cmem_sp) qbs_concat_cmem(size);
    qbs *newstr;
    newstr=qbs_new_descriptor();
    newstr->len=size;
    if ((qbs_cmem_sp+size)>cmem_sp) qbs_concat_cmem(size);
    newstr->chr=(uint8*)dblock+qbs_cmem_sp;
    qbs_cmem_sp+=size;
    newstr->in_cmem=1;
    if (qbs_cmem_list_nexti>qbs_cmem_list_lasti) qbs_cmem_concat_list();
    newstr->listi=qbs_cmem_list_nexti; qbs_cmem_list[newstr->listi]=(ptrszint)newstr; qbs_cmem_list_nexti++;
    if (tmp){
        if (qbs_tmp_list_nexti>qbs_tmp_list_lasti) qbs_tmp_concat_list();
        newstr->tmplisti=qbs_tmp_list_nexti; qbs_tmp_list[newstr->tmplisti]=(ptrszint)newstr; qbs_tmp_list_nexti++;
        newstr->tmp=1;
        }else{
        //alloc string descriptor in DBLOCK (4 bytes)
        cmem_sp-=4; newstr->cmem_descriptor=(uint16*)(dblock+cmem_sp); if (cmem_sp<qbs_cmem_sp) error(514);
        newstr->cmem_descriptor_offset=cmem_sp;
        //update cmem_descriptor [length][offset]
        newstr->cmem_descriptor[0]=newstr->len; newstr->cmem_descriptor[1]=(uint16)(ptrszint)(newstr->chr-dblock);
    }
    return newstr;
}

qbs *qbs_new(int32,uint8);

qbs *qbs_new_txt(const char *txt){
    qbs *newstr;
    newstr=qbs_new_descriptor();
    if (!txt){//NULL pointer is converted to a 0-length string
        newstr->len=0;
        }else{
        newstr->len=strlen(txt);
    }
    newstr->chr=(uint8*)txt;
    if (qbs_tmp_list_nexti>qbs_tmp_list_lasti) qbs_tmp_concat_list();
    newstr->tmplisti=qbs_tmp_list_nexti; qbs_tmp_list[newstr->tmplisti]=(ptrszint)newstr; qbs_tmp_list_nexti++;
    newstr->tmp=1;
    newstr->readonly=1;
    return newstr;
}

qbs *qbs_new_txt_len(const char *txt,int32 len){
    qbs *newstr;
    newstr=qbs_new_descriptor();
    newstr->len=len;
    newstr->chr=(uint8*)txt;
    if (qbs_tmp_list_nexti>qbs_tmp_list_lasti) qbs_tmp_concat_list();
    newstr->tmplisti=qbs_tmp_list_nexti; qbs_tmp_list[newstr->tmplisti]=(ptrszint)newstr; qbs_tmp_list_nexti++;
    newstr->tmp=1;
    newstr->readonly=1;
    return newstr;
}







//note: qbs_new_fixed detects if string is in DBLOCK
qbs *qbs_new_fixed(uint8 *offset,uint32 size,uint8 tmp){
    qbs *newstr;
    newstr=qbs_new_descriptor();
    newstr->len=size;
    newstr->chr=offset;
    newstr->fixed=1;
    if (tmp){
        if (qbs_tmp_list_nexti>qbs_tmp_list_lasti) qbs_tmp_concat_list();
        newstr->tmplisti=qbs_tmp_list_nexti; qbs_tmp_list[newstr->tmplisti]=(ptrszint)newstr; qbs_tmp_list_nexti++;
        newstr->tmp=1;
        }else{
        //is it in DBLOCK?
        if ((offset>(cmem+1280))&&(offset<(cmem+66816))){
            //alloc string descriptor in DBLOCK (4 bytes)
            cmem_sp-=4; newstr->cmem_descriptor=(uint16*)(dblock+cmem_sp); if (cmem_sp<qbs_cmem_sp) error(515);
            newstr->cmem_descriptor_offset=cmem_sp;
            //update cmem_descriptor [length][offset]
            newstr->cmem_descriptor[0]=newstr->len; newstr->cmem_descriptor[1]=(uint16)(ptrszint)(newstr->chr-dblock);
        }
    }
    return newstr;
}

qbs *qbs_new(int32 size,uint8 tmp){
    static qbs *newstr;
    if ((qbs_sp+size+32)>qbs_data_size) qbs_concat(size+32);
    newstr=qbs_new_descriptor();
    newstr->len=size;
    newstr->chr=qbs_data+qbs_sp;
    qbs_sp+=size+32;
    if (qbs_list_nexti>qbs_list_lasti) qbs_concat_list();
    newstr->listi=qbs_list_nexti; qbs_list[newstr->listi]=(ptrszint)newstr; qbs_list_nexti++;
    if (tmp){
        if (qbs_tmp_list_nexti>qbs_tmp_list_lasti) qbs_tmp_concat_list();
        newstr->tmplisti=qbs_tmp_list_nexti; qbs_tmp_list[newstr->tmplisti]=(ptrszint)newstr; qbs_tmp_list_nexti++;
        newstr->tmp=1;
    }
    return newstr;
}

void qbs_maketmp(qbs *str){
    //WARNING: assumes str is a non-tmp string in non-cmem
    if (qbs_tmp_list_nexti>qbs_tmp_list_lasti) qbs_tmp_concat_list();
    str->tmplisti=qbs_tmp_list_nexti; qbs_tmp_list[str->tmplisti]=(ptrszint)str; qbs_tmp_list_nexti++;
    str->tmp=1;
}

qbs *qbs_set(qbs *deststr,qbs *srcstr){
    int32 i;
    qbs *tqbs;
    //fixed deststr
    if (deststr->fixed){
        if (srcstr->len>=deststr->len){
            memcpy(deststr->chr,srcstr->chr,deststr->len);
            }else{
            memcpy(deststr->chr,srcstr->chr,srcstr->len);
            memset(deststr->chr+srcstr->len,32,deststr->len-srcstr->len);//pad with spaces
        }
        goto qbs_set_return;
    }
    //non-fixed deststr
    
    //can srcstr be aquired by deststr?
    if (srcstr->tmp){
        if (srcstr->fixed==0){
            if (srcstr->readonly==0){
                if (srcstr->in_cmem==deststr->in_cmem){
                    if (deststr->in_cmem){
                        //unlist deststr and acquire srcstr's list index
                        qbs_cmem_list[deststr->listi]=-1;
                        qbs_cmem_list[srcstr->listi]=(ptrszint)deststr;
                        deststr->listi=srcstr->listi;
                        }else{
                        //unlist deststr and acquire srcstr's list index
                        qbs_list[deststr->listi]=-1;
                        qbs_list[srcstr->listi]=(ptrszint)deststr;
                        deststr->listi=srcstr->listi;
                    }
                    
                    qbs_tmp_list[srcstr->tmplisti]=-1;
                    if (srcstr->tmplisti==(qbs_tmp_list_nexti-1)) qbs_tmp_list_nexti--;//correct last tmp index for performance
                    
                    deststr->chr=srcstr->chr;
                    deststr->len=srcstr->len;
                    qbs_free_descriptor(srcstr);
                    //update cmem_descriptor [length][offset]
                    if (deststr->cmem_descriptor){deststr->cmem_descriptor[0]=deststr->len; deststr->cmem_descriptor[1]=(uint16)(ptrszint)(deststr->chr-dblock);}
                    return deststr;//nb. This return cannot be changed to a goto qbs_set_return!
                }}}}
                
                //srcstr is equal length or shorter
                if (srcstr->len<=deststr->len){
                    memcpy(deststr->chr,srcstr->chr,srcstr->len);
                    deststr->len=srcstr->len;
                    goto qbs_set_return;
                }
                
                //srcstr is longer
                if (deststr->in_cmem){
                    if (deststr->listi==(qbs_cmem_list_nexti-1)){//last index
                        if (((ptrszint)deststr->chr+srcstr->len)<=(dblock+cmem_sp)){//space available
                            memcpy(deststr->chr,srcstr->chr,srcstr->len);
                            deststr->len=srcstr->len;  
                            qbs_cmem_sp=((ptrszint)deststr->chr)+(ptrszint)deststr->len-dblock;
                            goto qbs_set_return;
                        }
                        goto qbs_set_cmem_concat_required;
                    }
                    //deststr is not the last index so locate next valid index
                    i=deststr->listi+1;
                    qbs_set_nextindex:
                    if (qbs_cmem_list[i]!=-1){
                        tqbs=(qbs*)qbs_cmem_list[i];
                        if (tqbs==srcstr){
                            if (srcstr->tmp==1) goto skippedtmpsrcindex;
                        }
                        if ((deststr->chr+srcstr->len)>tqbs->chr) goto qbs_set_cmem_concat_required;
                        memcpy(deststr->chr,srcstr->chr,srcstr->len);
                        deststr->len=srcstr->len;
                        goto qbs_set_return;
                    }
                    skippedtmpsrcindex:
                    i++;
                    if (i!=qbs_cmem_list_nexti) goto qbs_set_nextindex;
                    //all next indexes invalid!
                    qbs_cmem_list_nexti=deststr->listi+1;//adjust nexti
                    if (((ptrszint)deststr->chr+srcstr->len)<=(dblock+cmem_sp)){//space available
                        memmove(deststr->chr,srcstr->chr,srcstr->len);//overlap possible due to sometimes aquiring srcstr's space
                        deststr->len=srcstr->len;
                        qbs_cmem_sp=((ptrszint)deststr->chr)+(ptrszint)deststr->len-dblock;
                        goto qbs_set_return;
                    }
                    qbs_set_cmem_concat_required:
                    //srcstr could not fit in deststr
                    //"realloc" deststr
                    qbs_cmem_list[deststr->listi]=-1;//unlist
                    if ((qbs_cmem_sp+srcstr->len)>cmem_sp){//must concat!
                        qbs_concat_cmem(srcstr->len);
                    }
                    if (qbs_cmem_list_nexti>qbs_cmem_list_lasti) qbs_cmem_concat_list();
                    deststr->listi=qbs_cmem_list_nexti;
                    qbs_cmem_list[qbs_cmem_list_nexti]=(ptrszint)deststr; qbs_cmem_list_nexti++; //relist
                    deststr->chr=(uint8*)dblock+qbs_cmem_sp;
                    deststr->len=srcstr->len;
                    qbs_cmem_sp+=deststr->len;
                    memcpy(deststr->chr,srcstr->chr,srcstr->len);
                    goto qbs_set_return;
                }
                
                
                //not in cmem
                if (deststr->listi==(qbs_list_nexti-1)){//last index
                    if (((ptrszint)deststr->chr+srcstr->len)<=((ptrszint)qbs_data+qbs_data_size)){//space available
                        memcpy(deststr->chr,srcstr->chr,srcstr->len);
                        deststr->len=srcstr->len;
                        qbs_sp=((ptrszint)deststr->chr)+(ptrszint)deststr->len-(ptrszint)qbs_data;
                        goto qbs_set_return;
                    }
                    goto qbs_set_concat_required;
                }
                //deststr is not the last index so locate next valid index
                i=deststr->listi+1;
                qbs_set_nextindex2:
                if (qbs_list[i]!=-1){
                    tqbs=(qbs*)qbs_list[i];
                    if (tqbs==srcstr){
                        if (srcstr->tmp==1) goto skippedtmpsrcindex2;
                    }
                    if ((deststr->chr+srcstr->len)>tqbs->chr) goto qbs_set_concat_required;
                    memcpy(deststr->chr,srcstr->chr,srcstr->len);
                    deststr->len=srcstr->len;
                    goto qbs_set_return;
                }
                skippedtmpsrcindex2:
                i++;
                if (i!=qbs_list_nexti) goto qbs_set_nextindex2;
                //all next indexes invalid!
                
                qbs_list_nexti=deststr->listi+1;//adjust nexti 
                if (((ptrszint)deststr->chr+srcstr->len)<=((ptrszint)qbs_data+qbs_data_size)){//space available
                    memmove(deststr->chr,srcstr->chr,srcstr->len);//overlap possible due to sometimes aquiring srcstr's space
                    deststr->len=srcstr->len;
                    qbs_sp=((ptrszint)deststr->chr)+(ptrszint)deststr->len-(ptrszint)qbs_data;
                    goto qbs_set_return;
                }
                
                qbs_set_concat_required:
                //srcstr could not fit in deststr
                //"realloc" deststr
                qbs_list[deststr->listi]=-1;//unlist
                if ((qbs_sp+srcstr->len)>qbs_data_size){//must concat!
                    qbs_concat(srcstr->len);
                }
                if (qbs_list_nexti>qbs_list_lasti) qbs_concat_list();
                deststr->listi=qbs_list_nexti;
                qbs_list[qbs_list_nexti]=(ptrszint)deststr; qbs_list_nexti++; //relist
                
                deststr->chr=qbs_data+qbs_sp;
                deststr->len=srcstr->len;
                qbs_sp+=deststr->len;
                memcpy(deststr->chr,srcstr->chr,srcstr->len);
                
                //(fall through to qbs_set_return)
                qbs_set_return:
                if (srcstr->tmp){//remove srcstr if it is a tmp string
                    qbs_free(srcstr);
                }
                //update cmem_descriptor [length][offset]
                if (deststr->cmem_descriptor){deststr->cmem_descriptor[0]=deststr->len; deststr->cmem_descriptor[1]=(uint16)(ptrszint)(deststr->chr-dblock);}
                return deststr;
}

qbs *qbs_add(qbs *str1,qbs *str2){
    qbs *tqbs;
    if (!str2->len) return str1;//pass on
    if (!str1->len) return str2;//pass on
    //may be possible to acquire str1 or str2's space but...
    //1. check if dest has enough space (because its data is already in the correct place)
    //2. check if source has enough space
    //3. give up
    //nb. they would also have to be a tmp, var. len str in ext memory!
    //brute force method...
    tqbs=qbs_new(str1->len+str2->len,1);
    memcpy(tqbs->chr,str1->chr,str1->len);
    memcpy(tqbs->chr+str1->len,str2->chr,str2->len);
    
    //exit(qbs_sp);
    if (str1->tmp) qbs_free(str1);
    if (str2->tmp) qbs_free(str2);
    return tqbs;
}

qbs *qbs_ucase(qbs *str){
    if (!str->len) return str;
    qbs *tqbs = NULL;
    if (str->tmp&&!str->fixed&&!str->readonly&&!str->in_cmem) {
        tqbs=str;
    }
    else {
        tqbs = qbs_new(str->len,1);
        memcpy(tqbs->chr,str->chr,str->len);
    }
    unsigned char *c=tqbs->chr;
    for (int32 i=0;i<str->len; i++) {
        if ((*c>='a')&&(*c<='z')) *c=*c&223;
        c++;
    }
    if (tqbs!=str&&str->tmp) qbs_free(str);
    return tqbs;
}

qbs *qbs_lcase(qbs *str){
    if (!str->len) return str;
    qbs *tqbs = NULL;
    if (str->tmp&&!str->fixed&&!str->readonly&&!str->in_cmem) {
        tqbs=str;
    }
    else {
        tqbs = qbs_new(str->len,1);
        memcpy(tqbs->chr,str->chr,str->len);
    }
    unsigned char *c=tqbs->chr;
    for (int32 i=0;i<str->len; i++) {
        if ((*c>='A')&&(*c<='Z')) *c=*c|32;
        c++;
    }
    if (tqbs!=str&&str->tmp) qbs_free(str);
    return tqbs;
}

qbs *func_chr(int32 value){
    qbs *tqbs;
    if ((value<0)||(value>255)){
        tqbs=qbs_new(0,1);
        error(5);
        }else{
        tqbs=qbs_new(1,1);
        tqbs->chr[0]=value;
    }
    return tqbs;
}


qbs *func_varptr_helper(uint8 type,uint16 offset){
    //*creates a 3 byte string using the values given
    qbs *tqbs;
    tqbs=qbs_new(3,1);
    tqbs->chr[0]=type;
    tqbs->chr[1]=offset&255;
    tqbs->chr[2]=offset>>8;
    return tqbs;
}

qbs *qbs_left(qbs *str,int32 l){
    if (l>str->len) l=str->len;
    if (l<0) l=0;
    if (l==str->len) return str;//pass on
    if (str->tmp){ if (!str->fixed){ if (!str->readonly){ if (!str->in_cmem){ str->len=l; return str; }}}}
    qbs *tqbs;
    tqbs=qbs_new(l,1);
    if (l) memcpy(tqbs->chr,str->chr,l);
    if (str->tmp) qbs_free(str);
    return tqbs;
}

qbs *qbs_right(qbs *str,int32 l){
    if (l>str->len) l=str->len;
    if (l<0) l=0;
    if (l==str->len) return str;//pass on
    if (str->tmp){ if (!str->fixed){ if (!str->readonly){ if (!str->in_cmem){
        str->chr=str->chr+(str->len-l);
        str->len=l; return str;
    }}}}
    qbs *tqbs;
    tqbs=qbs_new(l,1);
    if (l) memcpy(tqbs->chr,str->chr+str->len-l,l);
    tqbs->len=l;
    if (str->tmp) qbs_free(str);
    return tqbs;
}
qbs *func_space(int32 spaces){
    static qbs *tqbs;
    if (spaces<0) spaces=0;
    tqbs=qbs_new(spaces,1);
    if (spaces) memset(tqbs->chr,32,spaces);
    return tqbs;
}

qbs *func_string(int32 characters,int32 asciivalue){
    static qbs *tqbs;
    if (characters<0) characters=0;
    tqbs=qbs_new(characters,1);
    if (characters) memset(tqbs->chr,asciivalue&0xFF,characters);
    return tqbs;
}

void set_qbs_size(ptrszint *target_qbs,int32 newlength) {
    qbs_set((qbs*)(*target_qbs), func_space(newlength));
    return;
}

int32 func_instr(int32 start,qbs *str,qbs *substr,int32 passed){
    //QBNex difference: start can be 0 or negative
    //justification-start could be larger than the length of string to search in QBASIC
    static uint8 *limit,*base;
    static uint8 firstc;
    if (!passed) start=1;
    if (!str->len) return 0;
    if (start<1){
        start=1;
        if (!substr->len) return 0;
    }
    if (start>str->len) return 0;
    if (!substr->len) return start;
    if ((start+substr->len-1)>str->len) return 0;
    limit=str->chr+str->len;
    firstc=substr->chr[0];
    base=str->chr+start-1;
    nextchar:
    base=(uint8*)memchr(base,firstc,limit-base);
    if (!base) return 0;
    if ((base+substr->len)>limit) return 0;
    if (!memcmp(base,substr->chr,substr->len)) return base-str->chr+1;
    base++;
    if ((base+substr->len)>limit) return 0;
    goto nextchar;
}

int32 func__instrrev(int32 start,qbs *str,qbs *substr,int32 passed){
    if (!str->len) return 0;
    if (substr->len>str->len) return 0;
    if (!passed) {
        if (substr->len==str->len) {
            if (!memcmp(str->chr,substr->chr,str->len)) return 1;
        }
        start=str->len-substr->len+1;
    }
    if (start<1){
        start=str->len-substr->len+1;
    }
    if (start>str->len) start=str->len-substr->len+1;
    if (!substr->len) return start-1;
    if ((start+substr->len-1)>str->len) start=str->len-substr->len+1;

    int32 searchForward=0,lastFound=0,result=0;
    do {
        searchForward=func_instr(searchForward+1,str,substr,1);
        if (searchForward>0) {
            lastFound=searchForward;
            if (lastFound<=start) result=lastFound;
            if (lastFound>start) break;
        }
    } while (searchForward>0);

    return result;
}

void sub_mid(qbs *dest,int32 start,int32 l,qbs* src,int32 passed){
    if (new_error) return;
    static int32 src_offset;
    if (!passed) l=src->len;
    src_offset=0;
    if (dest==nothingstring) return;//quiet exit, error has already been reported!
    if (start<1){
        l=l+start-1;
        src_offset=-start+1;//src_offset is a byte offset with base 0!
        start=1;
    }
    if (l<=0) return;
    if (start>dest->len) return;
    if ((start+l-1)>dest->len) l=dest->len-start+1;
    //start and l are now reflect a valid region within dest
    if (src_offset>=src->len) return;
    if (l>(src->len-src_offset)) l=src->len-src_offset;
    //src_offset and l now reflect a valid region within src
    if (dest==src){
        if ((start-1)!=src_offset) memmove(dest->chr+start-1,src->chr+src_offset,l);
        }else{
        memcpy(dest->chr+start-1,src->chr+src_offset,l);
    }
}

qbs *func_mid(qbs *str,int32 start,int32 l,int32 passed){
    static qbs *tqbs;
    if (passed){
        if (start<1) {l=l-1+start; start=1;}
        if ((l>=1)&&(start<=str->len)){
            if ((start+l)>str->len) l=str->len-start+1;
            }else{
            l=0; start=1;//nothing!
        }
        }else{
        if (start<1) start=1;
        l=str->len-start+1;
        if (l<1){
            l=0; start=1;//nothing!
        }
    }
    if ((start==1)&&(l==str->len)) return str;//pass on
    if (str->tmp){ if (!str->fixed){ if (!str->readonly){ if (!str->in_cmem){//acquire
        str->chr=str->chr+(start-1);
        str->len=l;
        return str;
    }}}}
    tqbs=qbs_new(l,1);
    if (l) memcpy(tqbs->chr,str->chr+start-1,l);
    if (str->tmp) qbs_free(str);
    return tqbs;
}

qbs *qbs_ltrim(qbs *str){
    if (!str->len) return str;//pass on
    if (*str->chr!=32) return str;//pass on
    if (str->tmp){ if (!str->fixed){ if (!str->readonly){ if (!str->in_cmem){//acquire?
        qbs_ltrim_nextchar:
        if (*str->chr==32){
            str->chr++;
            if (--str->len) goto qbs_ltrim_nextchar;
        }
        return str;
    }}}}
    int32 i;
    i=0;
    qbs_ltrim_nextchar2: if (str->chr[i]==32) {i++; if (i<str->len) goto qbs_ltrim_nextchar2;}
    qbs *tqbs;
    tqbs=qbs_new(str->len-i,1);
    if (tqbs->len) memcpy(tqbs->chr,str->chr+i,tqbs->len);
    if (str->tmp) qbs_free(str);
    return tqbs;
}

qbs *qbs_rtrim(qbs *str){
    if (!str->len) return str;//pass on
    if (str->chr[str->len-1]!=32) return str;//pass on
    if (str->tmp){ if (!str->fixed){ if (!str->readonly){ if (!str->in_cmem){//acquire?
        qbs_rtrim_nextchar:
        if (str->chr[str->len-1]==32){
            if (--str->len) goto qbs_rtrim_nextchar;
        }
        return str;
    }}}}
    int32 i;
    i=str->len;
    qbs_rtrim_nextchar2: if (str->chr[i-1]==32) {i--; if (i) goto qbs_rtrim_nextchar2;}
    //i is the number of characters to keep
    qbs *tqbs;
    tqbs=qbs_new(i,1);
    if (i) memcpy(tqbs->chr,str->chr,i);
    if (str->tmp) qbs_free(str);
    return tqbs;
}

qbs *qbs__trim(qbs *str){
    return qbs_rtrim(qbs_ltrim(str));
}

int32 func__str_nc_compare(qbs *s1, qbs *s2) {
    int32 limit, l1, l2;
    int32 v1, v2;
    unsigned char *c1=s1->chr, *c2=s2->chr;
    
    l1 = s1->len; l2 = s2->len;  //no need to get the length of these strings multiple times.
    if (!l1) {   
        if (l2) return -1; else return 0;  //if one is a null string we known the answer already.
    }
    if (!l2) return 1;
    if (l1<=l2) limit = l1; else limit = l2; //our limit is going to be the length of the smallest string.
    
    for (int32 i=0;i<limit; i++) {  //check the length of our string
        v1=*c1;v2=*c2;
        if ((v1>64)&&(v1<91)) v1=v1|32;
        if ((v2>64)&&(v2<91)) v2=v2|32;
        if (v1<v2) return -1;
        if (v1>v2) return 1;
        c1++;
        c2++;
    }
    
    if (l1<l2) return -1; 
    if (l1>l2) return 1;
    return 0;
}

int32 func__str_compare(qbs *s1, qbs *s2) {
    int32 i, limit, l1, l2;
    l1 = s1->len; l2 = s2->len;  //no need to get the length of these strings multiple times.
    if (!l1) {   
        if (l2) return -1; else return 0;  //if one is a null string we known the answer already.
    }
    if (!l2) return 1;
    if (l1<=l2) limit = l1; else limit = l2; 
    i=memcmp(s1->chr,s2->chr,limit); 
    if (i<0) return -1;
    if (i>0) return 1; 
    if (l1<l2) return -1; 
    if (l1>l2) return 1;
    return 0;
}
int32 qbs_equal(qbs *str1,qbs *str2){
    if (str1->len!=str2->len) return 0;
    if (memcmp(str1->chr,str2->chr,str1->len)==0) return -1;
    return 0;
}
int32 qbs_notequal(qbs *str1,qbs *str2){
    if (str1->len!=str2->len) return -1;
    if (memcmp(str1->chr,str2->chr,str1->len)==0) return 0;
    return -1;
}
int32 qbs_greaterthan(qbs *str2,qbs *str1){
    //same process as for lessthan; we just reverse the string order
    int32 i, limit, l1, l2;
    l1 = str1->len; l2 = str2->len;  
    if (!l1) if (l2) return -1; else return 0;
    if (l1<=l2) limit = l1; else limit = l2; 
    i=memcmp(str1->chr,str2->chr,limit); 
    if (i<0) return -1;
    if (i>0) return 0; 
    if (l1<l2) return -1;   
    return 0;
}
int32 qbs_lessthan(qbs *str1,qbs *str2){
    int32 i, limit, l1, l2;
    l1 = str1->len; l2 = str2->len;  //no need to get the length of these strings multiple times.
    if (!l1) if (l2) return -1; else return 0;  //if one is a null string we known the answer already.
    if (l1<=l2) limit = l1; else limit = l2; //our limit is going to be the length of the smallest string.
    i=memcmp(str1->chr,str2->chr,limit); //check only to the length of the shortest string
    if (i<0) return -1; //if the number is smaller by this point, say so
    if (i>0) return 0; // if it's larger by this point, say so
    //if the number is the same at this point, compare length.
    //if the length of the first one is smaller, then the string is smaller. Otherwise the second one is the same string, or longer.
    if (l1<l2) return -1;   
    return 0;
}
int32 qbs_lessorequal(qbs *str1,qbs *str2){
    //same process as lessthan, but we check to see if the lengths are equal here also.
    int32 i, limit, l1, l2;
    l1 = str1->len; l2 = str2->len; 
    if (!l1) return -1;  //if the first string has no length then it HAS to be smaller or equal to the second
    if (l1<=l2) limit = l1; else limit = l2;
    i=memcmp(str1->chr,str2->chr,limit); 
    if (i<0) return -1;
    if (i>0) return 0; 
    if (l1<=l2) return -1;  
    return 0;
}
int32 qbs_greaterorequal(qbs *str2,qbs *str1){
    //same process as for lessorequal; we just reverse the string order
    int32 i, limit, l1, l2;
    l1 = str1->len; l2 = str2->len; 
    if (!l1) return -1;
    if (l1<=l2) limit = l1; else limit = l2;
    i=memcmp(str1->chr,str2->chr,limit); 
    if (i<0) return -1;
    if (i>0) return 0; 
    if (l1<=l2) return -1;  
    return 0;
}

int32 qbs_asc(qbs *str,uint32 i){//uint32 speeds up checking for negative
    i--;
    if (i<str->len){
        return str->chr[i];
    }
    error(5);
    return 0;
}


int32 qbs_asc(qbs *str){
    if (str->len) return str->chr[0];
    error(5);
    return 0;
}

int32 qbs_len(qbs *str){
    return str->len;
}
template <typename T> static inline T qbs_cleanup(uint32 base, T passvalue){
    // Fast-path: no temp strings were created since last cleanup
    if (qbs_tmp_list_nexti == base) return passvalue;
    while (qbs_tmp_list_nexti > base) {
        qbs_tmp_list_nexti--;
        if (qbs_tmp_list[qbs_tmp_list_nexti] != -1)
            qbs_free((qbs*)qbs_tmp_list[qbs_tmp_list_nexti]);
    }
    return passvalue;
}
