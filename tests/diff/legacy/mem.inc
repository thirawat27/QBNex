// Verbatim copy of the legacy _MEM code from libqb.cpp (commit 545b859), test-only.
// Lines 1222-1284 (lock storage) and 27412-27486, 27537-27711 (_MEM functions).
namespace {
    constexpr uint64 MEM_LOCK_INVALID_ID = 1073741821;
    constexpr uint64 MEM_LOCK_INITIAL_ID = 1073741823; // this value should never be 0 or 1
    constexpr int32 MEM_LOCK_BLOCK_SIZE = 10000;
    constexpr std::size_t MEM_LOCK_FREED_RESERVE = 1000;
    
    mem_lock mem_lock_invalid{};
    std::vector<std::unique_ptr<mem_lock[]>> mem_lock_blocks;
    std::vector<mem_lock*> mem_lock_freed;
    std::unordered_map<mem_lock*, std::vector<uint8>> memnew_storage;
    
    void allocate_mem_lock_block();
    
    struct mem_lock_storage_initializer {
        mem_lock_storage_initializer(){
            mem_lock_blocks.reserve(4);
            mem_lock_freed.reserve(MEM_LOCK_FREED_RESERVE);
            allocate_mem_lock_block();
        }
    };
}

//QBNex memory blocks
uint64 mem_lock_id=MEM_LOCK_INITIAL_ID;
int32 mem_lock_max=MEM_LOCK_BLOCK_SIZE;
int32 mem_lock_next=0;
mem_lock *mem_lock_base=NULL;
mem_lock *mem_lock_tmp;

namespace {
    void allocate_mem_lock_block(){
        std::unique_ptr<mem_lock[]> block(new(std::nothrow) mem_lock[mem_lock_max]);
        if (!block) error(504);
        mem_lock_base=block.get();
        mem_lock_blocks.push_back(std::move(block));
        mem_lock_next=0;
    }
    
    mem_lock_storage_initializer mem_lock_storage;
}

void new_mem_lock(){
    if (!mem_lock_freed.empty()){
        mem_lock_tmp=mem_lock_freed.back();
        mem_lock_freed.pop_back();
        }else{
        if (mem_lock_next==mem_lock_max){allocate_mem_lock_block();}
        mem_lock_tmp=&mem_lock_base[mem_lock_next++];
    }
    mem_lock_tmp->id=++mem_lock_id;
    mem_lock_tmp->type=0;
    mem_lock_tmp->offset=NULL;
}

void free_mem_lock(mem_lock *lock){
    lock->id=0;//invalidate lock
    if (lock->type==1) free(lock->offset);//malloc type
    lock->type=0;
    lock->offset=NULL;
    //add to freed list
    mem_lock_freed.push_back(lock);
}

        void sub__memfree(void *mem){
            //1:malloc: memory will be freed if it still exists
            //2:images: will not be freed, no action will be taken
            //exists?
            if (((mem_block*)(mem))->lock_offset==NULL){error(309); return;}
            if (((mem_lock*)(((mem_block*)(mem))->lock_offset))->id!=((mem_block*)(mem))->lock_id){error(307); return;}//memory has been freed
            if ( ((mem_lock*)(((mem_block*)(mem))->lock_offset))->type ==0){//no security
                free_mem_lock( (mem_lock*)((mem_block*)(mem))->lock_offset );
            }
            if ( ((mem_lock*)(((mem_block*)(mem))->lock_offset))->type ==1){//malloc
                memnew_storage.erase((mem_lock*)((mem_block*)(mem))->lock_offset);
                free_mem_lock( (mem_lock*)((mem_block*)(mem))->lock_offset );
            }
            //note: type 2(image) is freed when the image is freed
            //invalidate caller's mem structure (avoids misconception that _MEMFREE failed)
            ((mem_block*)(mem))->lock_id=MEM_LOCK_INVALID_ID;
        }
        
        
        extern mem_block func__mem_at_offset(ptrszint offset,ptrszint size){
            static mem_block b;
            new_mem_lock();
            mem_lock_tmp->type=0;//unsecured
            b.lock_offset=(ptrszint)mem_lock_tmp; b.lock_id=mem_lock_id;
            b.offset=offset;
            b.size=size;
            b.type=16384;//_MEMNEW type
            b.elementsize=1;
            b.image=-1;
            if ((size<0)||new_error){
                b.type=0;
                b.size=0;
                b.offset=0;
                if (size<0) error(301);
            }
            return b;
        }
        
        mem_block func__memnew(ptrszint bytes){
            static mem_block b;
            new_mem_lock();
            b.lock_offset=(ptrszint)mem_lock_tmp;
            b.lock_id=mem_lock_id;
            b.type=16384;//_MEMNEW type
            b.elementsize=1;
            b.image=-1;
            if (new_error){
                b.type=0;
                b.offset=0;
                b.size=0;
                mem_lock_tmp->type=0;
                return b;
            }
            
            if (bytes<0){
                //still create a block, but an invalid one and generate an error
                error(5);
                b.offset=0;
                b.size=0;
                mem_lock_tmp->type=0;
                }else{
                if (!bytes){
                    b.offset=1;//non-zero=success
                    b.size=0;
                    }else{
                    std::vector<uint8> &buffer=memnew_storage[mem_lock_tmp];
                    buffer.assign(static_cast<size_t>(bytes),0);
                    b.offset=(ptrszint)(buffer.empty()?NULL:buffer.data());
                    if (!b.offset){b.size=0; mem_lock_tmp->type=0; memnew_storage.erase(mem_lock_tmp);} else {b.size=bytes; mem_lock_tmp->type=1; mem_lock_tmp->offset=(void*)b.offset;}
                }
            }
            return b;
        }
        
        
        int32 func__memexists(void* void_blk){
            static mem_block *blk;
            blk=(mem_block*)void_blk;
            if ( ((mem_block*)(blk))->lock_offset==NULL ) return 0;
            if ( ((mem_lock*)(((mem_block*)(blk))->lock_offset))->id == ((mem_block*)(blk))->lock_id ) return -1;
            return 0;
        }
        
        void *func__memget(mem_block* blk,ptrszint off,ptrszint bytes){
            //checking A
            if ( ((mem_block*)(blk))->lock_offset==NULL ){error(309); goto fail;}
            //checking B
            if ( 
            off< ((mem_block*)(blk))->offset || (off+bytes)> (((mem_block*)(blk))->offset+((mem_block*)(blk))->size) ||
            ((mem_lock*)(((mem_block*)(blk))->lock_offset))->id!=((mem_block*)(blk))->lock_id
            ){
                //error reporting
                if ( ((mem_lock*)(((mem_block*)(blk))->lock_offset))->id!=((mem_block*)(blk))->lock_id ){error(308); goto fail;}
                error(300); goto fail;
            }
            return (void*)off;
            //------------------------------------------------------------
            fail:
            static std::vector<uint8> fail_buffer_storage;
            fail_buffer_storage.assign(static_cast<size_t>(bytes),0);
            void *fail_buffer=fail_buffer_storage.empty()?NULL:fail_buffer_storage.data();
            if (!fail_buffer) error(518);//critical error: out of memory
            return fail_buffer;
        }
        
        
        void sub__memfill_nochecks(ptrszint doff,ptrszint dbytes,ptrszint soff,ptrszint sbytes){
            if (sbytes==1){
                memset((void*)doff,*(uint8*)soff,dbytes);
                return;
            }
            static ptrszint si;
            si=0;
            while(dbytes--){
                *(int8*)(doff++)=*(int8*)(soff+si++);
                if (si>=sbytes) si=0;
            }
        }
        
        void sub__memfill(mem_block* dblk,ptrszint doff,ptrszint dbytes,ptrszint soff,ptrszint sbytes){
            if ( ((mem_block*)(dblk))->lock_offset==NULL ){error(309); return;}
            if ( ((mem_lock*)(((mem_block*)(dblk))->lock_offset))->id!=((mem_block*)(dblk))->lock_id ){error(308); return;}
            if ( (dbytes<0) || (sbytes==0) ){error(301); return;}
            if ( doff< ((mem_block*)(dblk))->offset || (doff+dbytes)> (((mem_block*)(dblk))->offset+((mem_block*)(dblk))->size) ){error(300); return;}
            sub__memfill_nochecks(doff,dbytes,soff,sbytes);
        }
        
        void sub__memfill_1(mem_block* dblk,ptrszint doff,ptrszint dbytes,int8 val){
            sub__memfill(dblk,doff,dbytes,(ptrszint)&val,1);
        }
        void sub__memfill_nochecks_1(ptrszint doff,ptrszint dbytes,int8 val){
            sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,1);
        }
        void sub__memfill_2(mem_block* dblk,ptrszint doff,ptrszint dbytes,int16 val){
            sub__memfill(dblk,doff,dbytes,(ptrszint)&val,2);
        }
        void sub__memfill_nochecks_2(ptrszint doff,ptrszint dbytes,int16 val){
            sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,2);
        }
        void sub__memfill_4(mem_block* dblk,ptrszint doff,ptrszint dbytes,int32 val){
            sub__memfill(dblk,doff,dbytes,(ptrszint)&val,4);
        }
        void sub__memfill_nochecks_4(ptrszint doff,ptrszint dbytes,int32 val){
            sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,4);
        }
        void sub__memfill_8(mem_block* dblk,ptrszint doff,ptrszint dbytes,int64 val){
            sub__memfill(dblk,doff,dbytes,(ptrszint)&val,8);
        }
        void sub__memfill_nochecks_8(ptrszint doff,ptrszint dbytes,int64 val){
            sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,8);
        }
        void sub__memfill_SINGLE(mem_block* dblk,ptrszint doff,ptrszint dbytes,float val){
            sub__memfill(dblk,doff,dbytes,(ptrszint)&val,4);
        }
        void sub__memfill_nochecks_SINGLE(ptrszint doff,ptrszint dbytes,float val){
            sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,4);
        }
        void sub__memfill_DOUBLE(mem_block* dblk,ptrszint doff,ptrszint dbytes,double val){
            sub__memfill(dblk,doff,dbytes,(ptrszint)&val,8);
        }
        void sub__memfill_nochecks_DOUBLE(ptrszint doff,ptrszint dbytes,double val){
            sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,8);
        }
        
        static uint8 memfill_FLOAT_padding[]={0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};//32 null bytes
        void sub__memfill_FLOAT(mem_block* dblk,ptrszint doff,ptrszint dbytes,long double val){
            *(long double*)memfill_FLOAT_padding=val;
            sub__memfill(dblk,doff,dbytes,(ptrszint)memfill_FLOAT_padding,32);
        }
        void sub__memfill_nochecks_FLOAT(ptrszint doff,ptrszint dbytes,long double val){
            *(long double*)memfill_FLOAT_padding=val;
            sub__memfill_nochecks(doff,dbytes,(ptrszint)memfill_FLOAT_padding,32);
        }
        
        void sub__memfill_OFFSET(mem_block* dblk,ptrszint doff,ptrszint dbytes,ptrszint val){
            sub__memfill(dblk,doff,dbytes,(ptrszint)&val,sizeof(ptrszint));
        }
        void sub__memfill_nochecks_OFFSET(ptrszint doff,ptrszint dbytes,ptrszint val){
            sub__memfill_nochecks(doff,dbytes,(ptrszint)&val,sizeof(ptrszint));
        }
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        
        void sub__memcopy(void *sblk,ptrszint soff,ptrszint bytes,void *dblk,ptrszint doff){
            //checking A
            if ( ((mem_block*)(sblk))->lock_offset==NULL || ((mem_block*)(dblk))->lock_offset==NULL ){
                //error reporting
                if ( ((mem_block*)(sblk))->lock_offset==NULL && ((mem_block*)(dblk))->lock_offset==NULL ){error(312); return;}
                if ( ((mem_block*)(sblk))->lock_offset==NULL ){error(310); return;}
                error(311); return;
            }
            //checking B
            if ( bytes<0 ||
            soff< ((mem_block*)(sblk))->offset || (soff+bytes)> (((mem_block*)(sblk))->offset+((mem_block*)(sblk))->size) ||
            doff< ((mem_block*)(dblk))->offset || (doff+bytes)> (((mem_block*)(dblk))->offset+((mem_block*)(dblk))->size) ||
            ((mem_lock*)(((mem_block*)(sblk))->lock_offset))->id!=((mem_block*)(sblk))->lock_id ||
            ((mem_lock*)(((mem_block*)(dblk))->lock_offset))->id!=((mem_block*)(dblk))->lock_id
            ){
                //error reporting
                if (((mem_lock*)(((mem_block*)(sblk))->lock_offset))->id!=((mem_block*)(sblk))->lock_id && ((mem_lock*)(((mem_block*)(dblk))->lock_offset))->id!=((mem_block*)(dblk))->lock_id){error(313); return;}
                if (((mem_lock*)(((mem_block*)(sblk))->lock_offset))->id!=((mem_block*)(sblk))->lock_id){error(305); return;}
                if (((mem_lock*)(((mem_block*)(dblk))->lock_offset))->id!=((mem_block*)(dblk))->lock_id){error(306); return;}
                if (bytes<0){error(301); return;}
                if ( soff< ((mem_block*)(sblk))->offset || (soff+bytes)> (((mem_block*)(sblk))->offset+((mem_block*)(sblk))->size) ){
                    if ( doff< ((mem_block*)(dblk))->offset || (doff+bytes)> (((mem_block*)(dblk))->offset+((mem_block*)(dblk))->size) ){error(304); return;}
                    error(302);
                    return;
                }
                error(303); return;
            }
            memmove((char*)doff,(char*)soff,bytes);
        }
        
        
        mem_block func__mem(ptrszint offset,ptrszint size,int32 type,ptrszint elementsize,mem_lock *lock){
            static mem_block b;
            b.lock_offset=(ptrszint)lock; b.lock_id=lock->id;
            b.offset=offset;
            b.size=size;
            b.type=type;
            b.elementsize=elementsize;
            b.image=-1;
            return b;
        }
