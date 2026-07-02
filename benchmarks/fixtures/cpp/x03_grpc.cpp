#include <grpcpp/grpcpp.h>

std::shared_ptr<grpc::Channel> chan(const std::string &addr) {
    return grpc::CreateChannel(addr, grpc::InsecureChannelCredentials());
}
