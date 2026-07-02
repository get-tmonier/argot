#include <opencv2/core.hpp>

cv::Mat scale(const cv::Mat &src, double f) {
    return src * f;
}
