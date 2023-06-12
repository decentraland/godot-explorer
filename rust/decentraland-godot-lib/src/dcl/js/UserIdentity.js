

// message Snapshots {
//     string face256 = 1;
//     string body = 2;
// }

// message AvatarForUserData {
//     string body_shape = 1;
//     string skin_color = 2;
//     string hair_color = 3;
//     string eye_color = 4;
//     repeated string wearables = 5;
//     Snapshots snapshots = 6;
// }

// message UserData {
//     string display_name = 1;
//     optional string public_key = 2;
//     bool has_connected_web3 = 3;
//     string user_id = 4;
//     int32 version = 5;
//     AvatarForUserData avatar = 6;
// }


// syntax = "proto3";
// package decentraland.kernel.apis;
// import "decentraland/common/sdk/user_data.proto";
// message GetUserDataRequest {}
// message GetUserDataResponse {
//     optional decentraland.common.sdk.UserData data = 1;
// }
// message GetUserPublicKeyRequest {}
// message GetUserPublicKeyResponse {
//     optional string address = 1;
// }
// service UserIdentityService {
//     // @deprecated, only available for SDK6 compatibility. UseGetUserData
//     rpc GetUserPublicKey(GetUserPublicKeyRequest) returns (GetUserPublicKeyResponse) {}
//     rpc GetUserData(GetUserDataRequest) returns (GetUserDataResponse) {}
// }


module.exports.GetUserPublicKey = async function (messages) {
    return {
        address: undefined
    }
}

module.exports.GetUserData = async function () {
    return {}
}


