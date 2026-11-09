// Verbatim legacy code from internal/c/qbx.cpp (commit ce58afd), used by tests/diff/diff_core.cpp.
// Do not edit: it is the reference the Rust port is compared against.

static std::vector<onstrig_struct> onstrig_storage(65536);
onstrig_struct *onstrig=onstrig_storage.data();//note: up to 256 controllers with up to 256 buttons each supported
int32 onstrig_inprogress=0;

void onstrig_setup(int32 i,int32 controller,int32 controller_passed,uint32 id,int64 pass){
    //note: pass is ignored by ids not requiring a pass value
    if (new_error) return;
    if (i<0||i>65535){error(5); return;}
    if (controller_passed){
        if (controller<1||controller>65535){error(5); return;}
        }else{
        controller=1; if (i&2){controller=2; i-=2;}
    }
    int32 button;
    button=(i>>2)+1;
    if (i&1){error(5); return;}//'currently down' state cannot be used as an ON STRIG event
    if (controller>256||button>256) return;//error-less exit for (currently) unsupported ranges
    i=(controller-1)*256+(button-1);//reindex
    onstrig[i].state=0;
    onstrig[i].pass=pass;
    onstrig[i].id=id;//id must be set last because it is the trigger variable
    if (device_last==0) func__devices();//init device interface (if not already setup)
}

void sub_strig(int32 i,int32 controller,int32 option,int32 passed){
    //ref: "[(?[,?])]{ON|OFF|STOP}"
    if (new_error) return;
    //Note: QuickBASIC ignores STRIG ON and STRIG OFF statements--the statements are provided for compatibility with earlier versions,
    //      Reference: http://www.antonis.de/qbebooks/gwbasman/strig.html
    //      QBNex makes STRIG ON/OFF/STOP change the checking status for all buttons
    static int32 i1,i2;
    if (passed>0){
        if (i<0||i>65535){error(5); return;}
        if (passed&2){
            if (controller<1||controller>65535){error(5); return;}
            }else{
            controller=1; if (i&2){controller=2; i-=2;}
        }
        static int32 button;
        button=(i>>2)+1;
        if (i&1){error(5); return;}//'currently down' state cannot be used as an ON STRIG event
        if (controller>256||button>256) return;//error-less exit for (currently) unsupported ranges
        i=(controller-1)*256+(button-1);//reindex
        i1=i; i2=i;
        }else{
        i1=0; i2=65535;
    }
    for (i=i1;i<=i2;i++){
        //ref: uint8 active;//0=OFF, 1=ON, 2=STOP
        if (option==1){//ON
            onstrig[i].active=1;
            if (onstrig[i].state) qbevent=1;
        }
        if (option==2){//OFF
            onstrig[i].active=0;
            onstrig[i].state=0;
        }
        if (option==3){//STOP
            onstrig[i].active=2;
            if (onstrig[i].state) onstrig[i].state=1;
        }
    }//i
}


static std::vector<onkey_struct> onkey_storage(32);
onkey_struct *onkey=onkey_storage.data();
int32 onkey_inprogress=0;

void onkey_setup(int32 i,uint32 id,int64 pass){
    //note: pass is ignored by ids not requiring a pass value
    if (new_error) return;
    if ((i<1)||(i>31)){error(5); return;}
    onkey[i].state=0;
    onkey[i].pass=pass;
    onkey[i].id=id;//id must be set last because it is the trigger variable
}

void sub_key(int32 i,int32 option){
    //ref: "(?){ON|OFF|STOP}"
    if (new_error) return;
    if ((i<0)||(i>31)){error(5); return;}
    static int32 i1,i2;
    i1=i; i2=i;
    if (!i){i1=i; i2=31;}//set all keys!
    for (i=i1;i<=i2;i++){
        //ref: uint8 active;//0=OFF, 1=ON, 2=STOP
        if (option==1){//ON
            onkey[i].active=1;
            if (onkey[i].state) qbevent=1;
        }
        if (option==2){//OFF
            onkey[i].active=0;
            onkey[i].state=0;
        }
        if (option==3){//STOP
            onkey[i].active=2;
            if (onkey[i].state) onkey[i].state=1;
        }
    }//i
}

int32 ontimer_nextfree=1;
std::vector<int32> ontimer_freelist;
std::vector<ontimer_struct> ontimer(1);
//note: index 0 of the above cannot be allocated/freed

static std::mutex ontimer_mutex;
static std::unique_ptr<std::unique_lock<std::mutex>> ontimer_pause_lock;

void stop_timers() {
    if (!ontimer_pause_lock){
        ontimer_pause_lock.reset(new std::unique_lock<std::mutex>(ontimer_mutex));
    }
}

void start_timers() {
    ontimer_pause_lock.reset();
}

static void freetimer_locked(int32 i){
    ontimer[i].allocated=0;
    ontimer[i].id=0;
    ontimer_freelist.push_back(i);
}

int32 func__freetimer(){
    if (new_error) return 0;
    std::lock_guard<std::mutex> lock(ontimer_mutex);
    int32 i;
    if (!ontimer_freelist.empty()){
        i=ontimer_freelist.back();
        ontimer_freelist.pop_back();
        }else{
        i=ontimer_nextfree;
        ontimer.resize(static_cast<size_t>(ontimer_nextfree)+1);
        ontimer[i].state=0;//state is not set to 0 if reusing an existing index as event could still be in progress
        ontimer_nextfree++;
    }
    ontimer[i].active=0;
    ontimer[i].id=0;
    ontimer[i].allocated=1;
    return i;
}

void freetimer(int32 i){
    std::lock_guard<std::mutex> lock(ontimer_mutex);
    freetimer_locked(i);
}

void ontimer_setup(int32 i,double sec,uint32 id,int64 pass){
    //note: pass is ignored by ids not requiring a pass value
    if (new_error) return;
    std::lock_guard<std::mutex> lock(ontimer_mutex);
    if ((i<0)||(i>=ontimer_nextfree)){error(5); return;}
    if (!ontimer[i].allocated){error(5); return;}
    if (ontimer[i].state==1) ontimer[i].state=0;//retract prev event if not in progress
    ontimer[i].seconds=sec;
    ontimer[i].pass=pass;
    ontimer[i].last_time=0;
    ontimer[i].id=id;//id must be set last because it is the trigger variable
}

void sub_timer(int32 i,int32 option,int32 passed){
    //ref: "[(?)]{ON|OFF|STOP|FREE}"
    if (new_error) return;
    if (!passed) i=0;
    std::lock_guard<std::mutex> lock(ontimer_mutex);
    if ((i<0)||(i>=ontimer_nextfree)){error(5); return;}
    if (!ontimer[i].allocated){error(5); return;}
    //ref: uint8 active;//0=OFF, 1=ON, 2=STOP
    if (option==1){//ON
        ontimer[i].active=1;
        return;
    }
    if (option==2){//OFF
        ontimer[i].active=0;
        if (ontimer[i].state==1) ontimer[i].state=0;//retract event if not in progress
        ontimer[i].last_time=0;//when ON is next used, event will be triggered immediately
        return;
    }
    if (option==3){//STOP
        ontimer[i].active=2;
        return;
    }
    if (option==4){//FREE
        if (i==0){error(5); return;}
        ontimer[i].active=0;
        if (ontimer[i].state==1) ontimer[i].state=0;//retract event if not in progress
        freetimer_locked(i);
        //note: if an event is still in progress, it will set state to 0 when it finishes
        //      which may delay the first instance of this index if it is immediately reused
        return;
    }
}



void TIMERTHREAD(){
    int32 i;
    double time_now = 100000.0;
    while(1){
        {
            std::lock_guard<std::mutex> lock(ontimer_mutex);
            time_now=((double)GetTicks())*0.001;
            for (i=0;i<ontimer_nextfree;i++){
                if (ontimer[i].allocated){
                    if (ontimer[i].id){
                        if (ontimer[i].active){
                            if (!ontimer[i].state){
                                if (time_now-ontimer[i].last_time>ontimer[i].seconds){
                                    if (!ontimer[i].last_time){
                                        ontimer[i].last_time=time_now;
                                        }else{
                                        //keep measured time for accurate number of calls overall
                                        ontimer[i].last_time+=ontimer[i].seconds;
                                        //if difference between actual time and measured time is beyond 'seconds' set measured to actual
                                        if (fabs(time_now-ontimer[i].last_time)>=ontimer[i].seconds) ontimer[i].last_time=time_now;
                                        ontimer[i].state=1;
                                        qbevent=1;
                                    }
                                }//time check
                            }//state==0
                        }//active
                    }//id
                }//allocated
            }//i
        }
        Sleep(1);
        if (stop_program){exit_ok|=2; return;}//close thread #2
    }//while(1)
    return;
}

