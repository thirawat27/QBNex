// VERBATIM copy of prepare_environment_2d (libqb/gui.cpp 158-259, oracle 545b859). Test-only.
    void prepare_environment_2d(){//called prior to rendering 2D content
        
        //precalculate critical dimensions, offsets & ratios
        
        static int32 can_scale;//can the screen be scaled on the window
        can_scale=0;
        static int32 need_square_pixels;//do we need square_pixels? if not we can stretch the screen
        need_square_pixels=0;
        
        environment_2d__screen_smooth=0;
        
        environment_2d__letterbox=0;
        
        if (full_screen>0){//in full screen
            //reference: ---int32 full_screen_set=-1;//0(windowed),1(stretched/closest),2(1:1)---
            can_scale=1;
            if (full_screen==2) need_square_pixels=1;
            //note: 'letter-boxing' is only requred where the size of the window cannot be controlled, and the only place where this is the
            //      case is full screen mode _SQUAREPIXELS
            environment_2d__screen_smooth=fullscreen_smooth;
            }else{//windowed
            if (resize_auto>0){//1=STRETCH,2=SMOOTH
                can_scale=1;
                if (resize_auto==2) environment_2d__screen_smooth=1;
                //note: screen will fix its aspect ratio automatically, so there is no need to enforce squarepixels
            }
        }
        
        if (environment_2d__screen_width==environment__window_width &&
        environment_2d__screen_height==environment__window_height){
            //screen size matches window
            environment_2d__screen_x1=0;
            environment_2d__screen_y1=0;
            environment_2d__screen_x2=environment_2d__screen_width-1;
            environment_2d__screen_y2=environment_2d__screen_height-1;
            environment_2d__screen_x_scale=1.0f;
            environment_2d__screen_y_scale=1.0f;
            environment_2d__screen_scaled_width=environment_2d__screen_width;
            environment_2d__screen_scaled_height=environment_2d__screen_height;
            environment_2d__screen_smooth=0;//no smoothing required
            }else{
            //screen size does not match
            //calculate ratios
            static float window_ratio;
            static float screen_ratio;
            window_ratio=(float)environment__window_width/(float)environment__window_height;
            screen_ratio=(float)environment_2d__screen_width/(float)environment_2d__screen_height;
            if (can_scale==0){
                //screen will appear in the top-left of the window with blank space on the bottom & right
                environment_2d__screen_x1=0;
                environment_2d__screen_y1=0;
                environment_2d__screen_x2=environment_2d__screen_width-1;
                environment_2d__screen_y2=environment_2d__screen_height-1;
                goto cant_scale;
            }
            if (need_square_pixels==0||(window_ratio==screen_ratio)){
                //can stretch, no 'letter-box' required
                environment_2d__screen_x1=0;
                environment_2d__screen_y1=0;
                environment_2d__screen_x2=environment__window_width-1;
                environment_2d__screen_y2=environment__window_height-1;
                }else{
                //'letter-box' required
                //this section needs revision
                static float x_scale,y_scale;
                static int32 x1,y1,x2,y2,z,x_limit,y_limit,x_offset,y_offset;
                //x_scale=(float)environment_2d__screen_width/(float)environment__window_width;
                //y_scale=(float)environment_2d__screen_height/(float)environment__window_height;
                //x_offset=0; y_offset=0;
                
                x1=0; y1=0; x2=environment__window_width-1; y2=environment__window_height-1;
                //x_limit=x2; y_limit=y2;
                if (window_ratio>screen_ratio){
                    //pad sides
                    z=(float)environment__window_height*screen_ratio;//new width
                    x1=environment__window_width/2-z/2;
                    x2=x1+z-1;
                    environment_2d__letterbox=1;//vertical black stripes required
                    //x_offset=-x1; x_scale=(float)environment_2d__screen_width/(float)z; x_limit=z-1;
                    }else{
                    //pad top/bottom
                    z=(float)environment__window_width/screen_ratio;//new height
                    y1=environment__window_height/2-z/2;
                    y2=y1+z-1;
                    environment_2d__letterbox=2;//horizontal black stripes required
                    //y_offset=-y1; y_scale=(float)environment_2d__screen_height/(float)z; y_limit=z-1;
                }
                environment_2d__screen_x1=x1;
                environment_2d__screen_y1=y1;
                environment_2d__screen_x2=x2;
                environment_2d__screen_y2=y2;
            }
            cant_scale:
            environment_2d__screen_scaled_width=environment_2d__screen_x2-environment_2d__screen_x1+1;
            environment_2d__screen_scaled_height=environment_2d__screen_y2-environment_2d__screen_y1+1;
            environment_2d__screen_x_scale=(float)environment_2d__screen_scaled_width/(float)environment_2d__screen_width;
            environment_2d__screen_y_scale=(float)environment_2d__screen_scaled_height/(float)environment_2d__screen_height;
        }
        
    }//prepare_environment_2d
    
    
