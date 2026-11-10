// Verbatim legacy code from internal/c/libqb.cpp (commit ce58afd), used by tests/diff/diff_core.cpp.
// Do not edit: it is the reference the Rust port is compared against.

uint32 mem_static_size;
extern uint8 *mem_static;
extern uint8 *mem_static_pointer;
extern uint8 *mem_static_limit;

namespace {
    std::vector<std::unique_ptr<uint8[]>> mem_static_blocks;

    void allocate_mem_static_block(uint32 size){
        std::unique_ptr<uint8[]> block(new(std::nothrow) uint8[size]);
        if (!block) error(504);
        mem_static=block.get();
        mem_static_blocks.push_back(std::move(block));
        mem_static_size=size;
        mem_static_pointer=mem_static;
        mem_static_limit=mem_static+mem_static_size;
    }
}

uint8 *mem_static_malloc(uint32 size){
    size+=7; size-=(size&7);//align to 8 byte boundry
    if ((mem_static_pointer+=size)<mem_static_limit) return mem_static_pointer-size;
    mem_static_size=(mem_static_size<<1)+size;
    allocate_mem_static_block(mem_static_size);
    mem_static_pointer=mem_static+size;
    return mem_static_pointer-size;
}
void mem_static_restore(uint8* restore_point){
    if ((restore_point>=mem_static)&&(restore_point<=mem_static_limit)){
        mem_static_pointer=restore_point;
        }else{
        //if restore_point is not in the current block, use t=start of current block as a new base
        mem_static_pointer=mem_static;
    }
}


struct cmem_dynamic_link_type{
    uint8 *offset;
    uint8 *top;
    uint32 size;
    uint32 i;
    cmem_dynamic_link_type *next;
};
cmem_dynamic_link_type cmem_dynamic_link[147136+1]; //+1 is added because array is used from index 1

//i=cmem_dynamic_next_link++; if (i>=147136) error(257);//not enough blocks
//newlink=(cmem_dynamic_link_type*)&cmem_dynamic_link[i];

cmem_dynamic_link_type *cmem_dynamic_link_first=NULL;
int32 cmem_dynamic_next_link=0;
int32 cmem_dynamic_free_link=0;
uint32 cmem_dynamic_free_list[147136];
uint8 *cmem_dynamic_malloc(uint32 size){
    static int32 i;
    static uint8 *top;
    static cmem_dynamic_link_type *link;
    static cmem_dynamic_link_type *newlink;
    static cmem_dynamic_link_type *prev_link;
    if (size>65536) error(505);//>64K
    //to avoid mismatches between offsets, all 0-byte blocks are given the special offset A000h (the top of the heap)
    if (!size) return(&cmem[0]+655360);//top of heap
    //forces blocks to be multiples of 16 bytes so they align with segment boundaries
    if (size&15) size=size-(size&15)+16;
    //is a space large enough between existing blocks available?
    //(if not, memory will be allocated at bottom of heap)
    top=&cmem[0]+655360;//top is the base of the higher block
    prev_link=NULL;
    if (link=cmem_dynamic_link_first){
        cmem_dynamic_findspace:
        if ((top-link->top)>=size){ //gpf
            //found free space
            goto cmem_dynamic_make_new_link;
        }
        prev_link=link; top=link->offset;//set top to the base of current block for future comparisons
        if (link=link->next) goto cmem_dynamic_findspace;
    }
    //no space between existing blocks is large enough, alloc below 'top'
    if ((top-cmem_static_pointer)<size) error(506);//a large enough block cannot be created!
    cmem_dynamic_base=top-size;
    //get a new link index
    cmem_dynamic_make_new_link:
    if (cmem_dynamic_free_link){
        i=cmem_dynamic_free_list[cmem_dynamic_free_link--];
        }else{
        i=cmem_dynamic_next_link++; if (i>=147136) error(507);//not enough blocks
    }
    newlink=(cmem_dynamic_link_type*)&cmem_dynamic_link[i];
    //set link info
    newlink->i=i;
    newlink->offset=top-size;
    newlink->size=size;
    newlink->top=top;
    //attach below prev_link
    if (prev_link){
        newlink->next=prev_link->next;//NULL if none
        prev_link->next=newlink;
        }else{
        newlink->next=cmem_dynamic_link_first;//NULL if none
        cmem_dynamic_link_first=newlink;
    }
    return newlink->offset;
}

void cmem_dynamic_free(uint8 *block){
    static cmem_dynamic_link_type *link;
    static cmem_dynamic_link_type *prev_link;
    if (!cmem_dynamic_link_first) return;
    if (!block) return;
    if (block==(&cmem[0]+655360)) return;//to avoid mismatches between offsets, all 0-byte blocks are given the special offset A000h (the top of the heap)
    prev_link=NULL;
    link=cmem_dynamic_link_first;
    check_next:
    if (link->offset==block){
        //unlink
        if (prev_link){
            prev_link->next=link->next;
            }else{
            cmem_dynamic_link_first=link->next;
        }
        //free link
        cmem_dynamic_free_link++;
        cmem_dynamic_free_list[cmem_dynamic_free_link]=link->i;
        //memory freed successfully!
        return;
    }
    prev_link=link;
    if (link=link->next) goto check_next;
    return;
}

