// Verbatim copy of legacy libqb.cpp (commit 545b859) lines 22699-22711, 22985-22994, 23085-23362:
// struct tcp_connection, struct connection_struct, connection_new .. func__connected. Test-only.
        struct tcp_connection{
            #if !defined(DEPENDENCY_SOCKETS)
                #elif defined(QBNex_WINDOWS)
                SOCKET socket;
                #elif defined(QBNex_UNIX)
                int socket;
                #else
            #endif
            int32 port;//connection to host & clients only
            uint8 ip4[4];//connection to host only
            uint8* hostname;//clients only
            int connected;
        };
        struct connection_struct{
            int8 in_use;//0=not being used, 1=in use
            int8 protocol;//1=TCP/IP
            int8 type;//1=client, 2=host(listening), 3=host's connection from a client
            ptrszint stream;
            ptrszint handle;
            void *connection;
            //---------------------------------
            int32 port;
        };
        int32 connection_new(int32 method, qbs *info_in, int32 value){
            //method: 0=_OPENCLIENT [info=~"TCP/IP:12345:23.96.32.123], value=NULL"
            //        1=_OPENHOST [info=~"TCP/IP:12345", value=NULL]
            //        2=_OPENCONNECTION [info=NULL, value=host's handle]
            //returns: -1=invalid arguments passed
            //          0=failed to open
            //         >0=handle of successfully opened connection
            
            static int32 i,x;
            
            //generic division of parts
            static qbs *info_part[10+1];
            static qbs *str;
            static qbs *strz;
            static qbs *info;
            
            static int32 first_call=1;
            if (first_call){
                first_call=0;
                for(i=1;i<=10;i++){info_part[i]=qbs_new(0,0);}
                str=qbs_new(0,0);
                strz=qbs_new(1,0); strz->chr[0]=0;
                info=qbs_new(0,0);
            }//first call
            
            //split info string
            static int32 parts;
            parts=0;
            if ((method==0)||(method==1)){
                qbs_set(info,info_in);
                qbs_set(str,qbs_new_txt(":"));
                i=1;
                next_part:
                x=func_instr(i,info,str,1);
                if (x){
                    parts++; qbs_set(info_part[parts],func_mid(info,i,x-i,1));
                    i=x+1;
                    goto next_part;
                }
                parts++; qbs_set(info_part[parts],func_mid(info,i,NULL,NULL));
            }//split info string
            
            static double d;
            static int32 port;
            
            if ((method==0)||(method==1)){
                
                if (parts<2) return -1;
                if (qbs_equal(qbs_ucase(info_part[1]),qbs_new_txt("TCP/IP"))==0) {
                    if (qbs_equal(qbs_ucase(info_part[1]),qbs_new_txt("QBNEXIDE"))==0 || vwatch!=-1) {
                        return -1;
                    }
                }
                
                d=func_val(info_part[2]);
                port=qbr_double_to_long(d);//***assume*** port number is within valid range
                
                if (method==0){//_OPENCLIENT
                    if (parts!=3) return -1;
                    
                    static void *connection;
                    qbs_set(str,qbs_add(info_part[3],strz));
                    connection=tcp_client_open(str->chr,port);
                    if (!connection) return 0;
                    
                    static int32 my_handle; my_handle=list_add(special_handles);
                    static special_handle_struct *my_handle_struct; my_handle_struct=(special_handle_struct*)list_get(special_handles,my_handle);
                    static int32 my_stream; my_stream=list_add(stream_handles);
                    static stream_struct *my_stream_struct; my_stream_struct=(stream_struct*)list_get(stream_handles,my_stream);
                    static int32 my_connection; my_connection=list_add(connection_handles);
                    static connection_struct *my_connection_struct; my_connection_struct=(connection_struct*)list_get(connection_handles,my_connection);
                    my_handle_struct->type=1;//stream
                    my_handle_struct->index=(ptrszint)my_stream_struct;
                    my_stream_struct->type=1;//network
                    my_stream_struct->index=(ptrszint)my_connection_struct;
                    my_connection_struct->protocol=1;//tcp/ip
                    my_connection_struct->type=1;//client
                    my_connection_struct->connection=connection;
                    my_connection_struct->port=port;
                    
                    //init stream
                    my_stream_struct->in=NULL; my_stream_struct->in_size=0; my_stream_struct->in_limit=0;
                    
                    if (vwatch==-1) vwatch=my_handle;
                    return my_handle;
                }//client
                
                
                if (method==1){//_OPENHOST
                    if (parts!=2) return -1;

                    static void *connection;
                    connection=tcp_host_open(port);
                    if (!connection) return 0;
                    
                    static int32 my_handle; my_handle=list_add(special_handles);
                    static special_handle_struct *my_handle_struct; my_handle_struct=(special_handle_struct*)list_get(special_handles,my_handle);
                    static int32 my_connection; my_connection=list_add(connection_handles);
                    static connection_struct *my_connection_struct; my_connection_struct=(connection_struct*)list_get(connection_handles,my_connection);
                    my_handle_struct->type=2;//host listener
                    my_handle_struct->index=(ptrszint)my_connection_struct;
                    my_connection_struct->protocol=1;//tcp/ip
                    my_connection_struct->type=2;//host(listening)
                    my_connection_struct->connection=connection;
                    my_connection_struct->port=port;
                    return my_handle;
                    
                }
            }//0 or 1
            
            if (method==2){//_OPENCONNECTION
                static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,value); if (!sh) return -1;
                if (sh->type!=2) return -1;//listening host?
                static connection_struct *co; co=(connection_struct*)sh->index;
                static void *connection;
                connection=tcp_connection_open(co->connection);
                if (!connection) return 0;
                
                static int32 my_handle; my_handle=list_add(special_handles);
                static special_handle_struct *my_handle_struct; my_handle_struct=(special_handle_struct*)list_get(special_handles,my_handle);
                static int32 my_stream; my_stream=list_add(stream_handles);
                static stream_struct *my_stream_struct; my_stream_struct=(stream_struct*)list_get(stream_handles,my_stream);
                static int32 my_connection; my_connection=list_add(connection_handles);
                static connection_struct *my_connection_struct; my_connection_struct=(connection_struct*)list_get(connection_handles,my_connection);
                my_handle_struct->type=1;//stream
                my_handle_struct->index=(ptrszint)my_stream_struct;
                my_stream_struct->type=1;//network
                my_stream_struct->index=(ptrszint)my_connection_struct;
                my_connection_struct->protocol=1;//tcp/ip
                my_connection_struct->type=3;//host's client connection
                my_connection_struct->connection=connection;
                my_connection_struct->port=port;
                
                //init stream
                my_stream_struct->in=NULL; my_stream_struct->in_size=0; my_stream_struct->in_limit=0;
                
                return my_handle;
                
            }//_OPENCONNECTION
            
        }//connection_new
        
        
        //network prototype:
        
        
        
        int32 func__openclient(qbs* info){
            if (new_error) return 0;
            static int32 i;
            i=connection_new(0,info,NULL);
            if (i==-1){error(5); return 0;}
            if (i==0) return 0;
            return -1-i;
        }
        
        int32 func__openhost(qbs* info){
            if (new_error) return 0;
            static int32 i;
            i=connection_new(1,info,NULL);
            if (i==-1){error(5); return 0;}
            if (i==0) return 0;
            return -1-i;
        }
        
        int32 func__openconnection(int32 i){
            
            if (new_error) return 0;
            i=-(i+1);
            i=connection_new(2,NULL,i);
            if (i==-1){error(258); return 0;}//invalid handle
            if (i==0) return 0;//no new connections
            return -1-i;
        }
        
        
        
        qbs *func__connectionaddress(int32 i){
            static qbs *tqbs,*tqbs2,*str=NULL,*str2=NULL;
            static int32 x;
            if (new_error) goto error;
            if (!str) str=qbs_new(0,0);
            if (!str2) str2=qbs_new(0,0);
            
            if (i<0){
                x=-(i+1);
                static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh){error(52); goto error;}
                
                if (sh->type==2){//host listener
                    static connection_struct *cs; cs=(connection_struct*)sh->index;
                    if (cs->protocol==1){//TCP/IP
                        qbs_set(str,qbs_new_txt("TCP/IP:"));//network type
                        qbs_set(str,qbs_add(str,qbs_ltrim(qbs_str(cs->port))));//port
                        qbs_set(str,qbs_add(str,qbs_new_txt(":")));
                        tqbs2=WHATISMYIP();
                        if (tqbs2->len){qbs_set(str,qbs_add(str,tqbs2));
                            }else{
                            qbs_set(str,qbs_add(str,qbs_new_txt("127.0.0.1")));//localhost
                        }
                        return str;
                    }//TCP/IP
                }//host listener
                
                //client or connection to host
                if (sh->type==1){//stream
                    static stream_struct *ss; ss=(stream_struct*)sh->index;
                    if (ss->type==1){//network
                        static connection_struct *cs; cs=(connection_struct*)ss->index;
                        if (cs->protocol==1){//TCP/IP
                            if (cs->type==1||cs->type==3){//1=client, 2=host(listening), 3=host's connection from a client
                                
                                static tcp_connection *tcp; tcp=(tcp_connection*)cs->connection;
                                qbs_set(str,qbs_new_txt("TCP/IP:"));//network type
                                qbs_set(str,qbs_add(str,qbs_ltrim(qbs_str(tcp->port))));//port
                                qbs_set(str,qbs_add(str,qbs_new_txt(":")));
                                if (cs->type==3){
                                    qbs_set(str,qbs_add(str,qbs_ltrim(qbs_str(tcp->ip4[0]))));//ip
                                    qbs_set(str,qbs_add(str,qbs_new_txt(".")));
                                    qbs_set(str,qbs_add(str,qbs_ltrim(qbs_str(tcp->ip4[1]))));//ip
                                    qbs_set(str,qbs_add(str,qbs_new_txt(".")));
                                    qbs_set(str,qbs_add(str,qbs_ltrim(qbs_str(tcp->ip4[2]))));//ip
                                    qbs_set(str,qbs_add(str,qbs_new_txt(".")));
                                    qbs_set(str,qbs_add(str,qbs_ltrim(qbs_str(tcp->ip4[3]))));//ip
                                    }else{
                                    qbs_set(str,qbs_add(str,qbs_new_txt((char*)tcp->hostname)));
                                }
                                return str;
                            }
                        }//TCP/IP
                    }//network
                }//stream
                
            }//i<0
            error(52); goto error;
            
            error:
            tqbs=qbs_new(0,1);
            return tqbs;
            
        }
        
        int32 tcp_connected (void *connection){
            tcp_connection *tcp=(tcp_connection*)connection;
            #ifndef DEPENDENCY_SOCKETS
                return 0;
                #else
                return tcp->connected;
            #endif
        }
        
        int32 func__connected(int32 i){
            if (new_error) return 0;
            if (i<0){
                static int32 x;
                x=-(i+1);
                static special_handle_struct *sh; sh=(special_handle_struct*)list_get(special_handles,x); if (!sh) goto error;
                
                if (sh->type==2){//host listener
                    static connection_struct *cs; cs=(connection_struct*)sh->index;
                    if (cs->protocol==1){//TCP/IP
                        return -1;
                    }//TCP/IP
                }//host listener
                
                //client or connection to host
                if (sh->type==1){//stream
                    static stream_struct *ss; ss=(stream_struct*)sh->index;
                    if (ss->type==1){//network
                        static connection_struct *cs; cs=(connection_struct*)ss->index;
                        if (cs->protocol==1){//TCP/IP
                            return tcp_connected(cs->connection);
                        }//TCP/IP
                    }//network
                }//stream
                
            }//i<0
            error: error(52); return 0;
        }
