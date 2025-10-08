use crate::define::ErrorMess;
use windows::Win32::{Networking::WinSock::*};
use std::{net::Ipv4Addr};
//mod define;
use std::collections::HashMap;
//use rand::{Rng};
use crate::define;
pub fn init_socket() -> Result< SOCKET, ErrorMess> {
    let mut wsa_data = WSADATA::default();
    let i_result = unsafe { WSAStartup(0x0202, &mut wsa_data) };
    if i_result != 0 {
        unsafe {WSACleanup()};
        let code = unsafe {WSAGetLastError().0};
        return Err(ErrorMess::WsaError(code))
    }
    let sock = match unsafe { socket(2, WINSOCK_SOCKET_TYPE(1), 6) } {
        Ok(socket) => socket,
        Err(_e) => {
            let code = unsafe { WSAGetLastError().0 };
            return Err(ErrorMess::WsaError(code))
        }
    };
    println!("tạo xong socket");
    Ok(sock)
}
pub fn create_server_addr() -> SOCKADDR_IN {
    let ip = Ipv4Addr::new(127, 0, 0, 1);
    let ip_u32 = u32::from(ip).to_be();  //địa chỉ và cổng luôn lưu theo chuẩn big-endian.
    let server_addr: SOCKADDR_IN = SOCKADDR_IN {
        sin_family: AF_INET,
        sin_port: unsafe {htons(9999)},
        // struct của in_addr gồm 3 union tương ứng 3 cách đọc vùng bộ nhớ 
        // S_addr là đọc theo u32
        sin_addr:  IN_ADDR { S_un: IN_ADDR_0 {S_addr: ip_u32} },
        sin_zero: [0; 8],
    };
    server_addr
}
pub fn bind_socket(sock: SOCKET, server_addr: SOCKADDR_IN) -> Result <(), ErrorMess>{
    // tham số name là con trỏ tới *const SOCKADDR, mà hiện tại socket đang dùng struct SOCKADD_IN
    // cần cast sang 
    let i_result = unsafe {
        bind(sock, 
                &server_addr as *const SOCKADDR_IN as *const SOCKADDR,  
                std::mem::size_of::<SOCKADDR_IN>() as i32,
        )
    };
    if i_result == SOCKET_ERROR {
        unsafe {closesocket(sock)};
        unsafe { WSACleanup() };
        let code: i32 = unsafe {  WSAGetLastError().0 };
        return Err(ErrorMess::WsaError(code))
    }
    Ok(())
}
pub fn setup_tcp_listen (sock: SOCKET) -> Result<(), ErrorMess> {
    let i_result = unsafe { listen (sock, SOMAXCONN as i32) };
    if i_result == SOCKET_ERROR {
          unsafe {closesocket(sock)}; unsafe { WSACleanup() };
        let code: i32 = unsafe {  WSAGetLastError().0 };
        return Err(ErrorMess::WsaError(code))
    }
    Ok(())
}
pub fn accept_client (sock: SOCKET) -> Result<SOCKET, ErrorMess> {
    let socket_client = match unsafe {accept(sock, None,  None)}
    {
        Ok(socket ) => socket,
        Err(_e) => {
            define::dbg_print(format!("loi ham accept").to_string().as_str());
            let code = unsafe {WSAGetLastError().0};
            define::dbg_print(format!("loi ham accept {code}").to_string().as_str());
            match code {
                10004 => return Err(ErrorMess::ListenerClosed),
                _ => {
                    define::dbg_print(format!("loi ham accept {code}").to_string().as_str());
                    return Err(ErrorMess::WsaError(code)) 
                }
            };
        }
    };
    Ok(socket_client)
}
pub fn convert_bytes_to_u32(bytes: &[u8]) -> Result<u32, ErrorMess> {
    if bytes.len() == 4 {
        // map_err sẽ chạy nếu kết quả trước đó khác Ok, bên trong là 1 closure chuyển kiểu Err T->F
        let arr: [u8; 4] = bytes.try_into().map_err(|_| ErrorMess::ConvertError)?;
        Ok(u32::from_be_bytes(arr))
    } else {
        define::dbg_print("lỗi khi convert_byte_to_u32");
        Err(ErrorMess::ConvertError)
    }
}
pub fn receive_mess (sock: SOCKET) ->  Result <(), ErrorMess>{
    let mut mess: HashMap<u32, Vec<u8>> = HashMap::new();
    let mut num_pack ;
    loop{
        let mut buf:[u8; 24] = [0u8; 24];
        let i_result = unsafe { recv(sock, 
                                        &mut buf, 
                                        SEND_RECV_FLAGS(0))};
        if i_result > 0 {

            let stt_new: u32 = convert_bytes_to_u32(&buf[0..4])?;

            num_pack = convert_bytes_to_u32(&buf[4..8])?;

            let _len_pack = convert_bytes_to_u32(&buf[8..12])?;

            let content: Vec<u8> = buf[12..].to_vec();

            mess.insert( stt_new , content);

            if stt_new == num_pack-1 {
                let mut mess_fin = vec![];
                for i in 0..num_pack as usize {
                    let stt = (i) as u32;
                    if let Some(mess_small) = mess.get(&stt){
                        mess_fin.push(mess_small.clone());
                    }else{
                        println!("lỗi tại stt {stt}");
                    }
                }
                let result = mess_fin.concat();
                let lpbuffer_read = String::from_utf8_lossy(&result).to_string();
                println!("{}",lpbuffer_read);
                send_again( &lpbuffer_read, sock)?;
                num_pack = 0;
                mess.clear();

            }
            continue;
        } else if i_result == 0 {
            break;
        }
        else {
            let code = unsafe {WSAGetLastError().0};
            if code == 10054 {
                break;
            }
            unsafe { WSACleanup() };
            unsafe { closesocket(sock)};
            return Err(ErrorMess::WsaError(code))
        }
    }
    Ok(())
}
pub fn send_again(input: &str, sock: SOCKET) -> Result<(), ErrorMess> {
    let buf: &[u8] = input.as_bytes();
    let total_pack = handle_pack_num(buf.len() as u32);

    for (stt, chunk) in buf.chunks(12).enumerate() {
        let chunked = vec![
            u32_to_vec(stt as u32),
            u32_to_vec(total_pack),
            u32_to_vec(chunk.len() as u32),
            chunk.to_vec()
        ].concat();
        let i_result = unsafe {send(sock, 
                                                &chunked, 
                                                SEND_RECV_FLAGS(0))};
        if i_result == SOCKET_ERROR {
            println!("client đóng kết nối");
            let code = unsafe {WSAGetLastError().0};
            unsafe { closesocket(sock) }; 
            unsafe { WSACleanup() };
            define::dbg_print("lỗi khi send data");
            return Err(ErrorMess::WsaError(code))
        }
    }
    Ok(())
}
pub fn _finish_socket (sock: SOCKET) -> Result<(), ErrorMess> {
    let i_result = unsafe {shutdown(sock, SD_SEND)};
    if i_result == SOCKET_ERROR {
       let code = unsafe {WSAGetLastError().0 };
        define::dbg_print(&format!("shutdown() failed: WSA error code {code}"));
    }
    let close_result = unsafe {closesocket(sock)};
    if close_result == SOCKET_ERROR {
        let code = unsafe {WSAGetLastError().0};
        define::dbg_print(&format!("closesocket() failed: WSA error code {code}"));
    } else {
        define::dbg_print("Socket closed successfully");
    }
    let cleanup_result = unsafe {WSACleanup()};
    if cleanup_result != 0 {
        let code = unsafe {WSAGetLastError().0};
        define::dbg_print(&format!("WSACleanup() failed: WSA error code {code}"));
    } else {
        define::dbg_print("Winsock cleanup complete");
    }
    Ok(())
}
pub fn u32_to_vec(num: u32) -> Vec<u8> {
    let vec_num: Vec<u8> = num.to_be_bytes().to_vec();
    vec_num
}
pub fn handle_pack_num(len_pack: u32) -> u32 {
    let mut number_pack =  ( len_pack / 12 ) as u32;
    let sodu = len_pack % 12;
    if sodu != 0 {
        number_pack += 1;
    }
    number_pack
}