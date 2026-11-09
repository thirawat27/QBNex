// Verbatim legacy code from internal/c/libqb.cpp (commit ce58afd), used by tests/diff/diff_core.cpp.
// Do not edit: it is the reference the Rust port is compared against.

void sub__delay(double seconds){
    double ms,base,elapsed,prev_now,now;//cannot be static
    base=GetTicks();
    if (new_error) return;
    if (seconds<0){error(5); return;}
    if (seconds>2147483.647){error(5); return;}
    ms=seconds*1000.0;
    now=base;//force first prev=... assignment to equal base
    recalculate:
    prev_now=now;
    now=GetTicks();
    elapsed=now-base;
    if (elapsed<0){//GetTicks looped
        base=now-(prev_now-base);//calculate new base
    }
    if (elapsed<ms){
        int64 wait;//cannot be static
        wait=ms-elapsed;
        if (!wait) wait=1;
        if (wait>=10){
            Sleep(9);
            evnt(0);//check for new events
            //recalculate time
            goto recalculate;
            }else{
            Sleep(wait);
        }
    }
}

void sub__limit(double fps){
    if (new_error) return;
    static double prev=0;
    double ms,now,elapsed;//cannot be static
    if (fps<=0.0){error(5); return;}
    ms=1000.0/fps;
    if (ms>60000.0){error(5); return;}//max. 1 min delay between frames allowed to avoid accidental lock-up of program
    recalculate:
    now=GetTicks();
    if (prev==0.0){//first call?
        prev=now;
        return;
    }
    if (now<prev){//value looped?
        prev=now;
        return;
    }
    elapsed=now-prev;//elapsed time since prev
    
    if (elapsed==ms){
        prev=prev+ms;
        return; 
    }
    
    if (elapsed<ms){
        int64 wait;//cannot be static
        wait=ms-elapsed;
        if (!wait) wait=1;
        if (wait>=10){
            Sleep(9);
            evnt(0);//check for new events
            }else{
            Sleep(wait);  
        }
        //recalculate time
        goto recalculate;
    }
    
    //too long since last call, adjust prev to current time
    //minor overshoot up to 32ms is recovered, otherwise time is re-seeded
    if (elapsed<=(ms+32.0)) prev=prev+ms; else prev=now;
}


// Tail of sub_sleep after the console branch (prev/ms are the function's locals):
void sub_sleep_tail(double prev,double ms){
    double now,elapsed;
        recalculate:
        wait:
        evnt(0);//handle general events
        //exit condition checks/events
        if (sleep_break) return;
        if (stop_program) return;
        if (ms<=0){//untimed SLEEP
            Sleep(9);
            goto wait;
        }
        now=GetTicks();
        if (now<prev){//value looped?
            return;
        }
        elapsed=now-prev;//elapsed time since prev
        if (elapsed<ms){
            int64 wait;//cannot be static
            wait=ms-elapsed;
            if (!wait) wait=1;
            if (wait>=10){
                Sleep(9);  
                //recalculate time
                goto recalculate;
                }else{
                Sleep(wait);
            }
        }
        return;
    }

