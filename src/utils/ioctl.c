#include <stdlib.h>
#include <fcntl.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/ioctl.h>
#include <sys/mman.h>

#include <linux/videodev2.h>

struct Buffer {
    void * start;
    size_t len;
};

int ffi_open_device(const char * path) {
    return open(path, O_RDWR);
}
int ffi_close_device(int fd) {
    return close(fd);
}
int ffi_support_video_streaming(unsigned int device_caps) {
    return (device_caps & V4L2_CAP_STREAMING) && (device_caps & V4L2_CAP_VIDEO_CAPTURE);
}

struct v4l2_capability * ffi_read_device_capability(int fd) {
    struct v4l2_capability * caps = (struct v4l2_capability *) malloc(sizeof(struct v4l2_capability));
    if (ioctl(fd, VIDIOC_QUERYCAP, caps) == -1) {
        free(caps);
        return NULL;
    }
    return caps;
}

struct v4l2_fmtdesc * ffi_format_info(int fd, unsigned int idx) {
    struct v4l2_fmtdesc * fmt = (struct v4l2_fmtdesc *) malloc(sizeof(struct v4l2_fmtdesc));
    memset(fmt, 0, sizeof(*fmt));
    fmt->index = idx;
    fmt->type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    if (ioctl(fd, VIDIOC_ENUM_FMT, fmt) == 0) {
        return fmt;
    } else {
        free(fmt);
        return NULL;
    }
}

int ffi_init_fmt(int fd, int width, int height) {
    struct v4l2_format fmt = {0};
    fmt.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    fmt.fmt.pix.width = width;
    fmt.fmt.pix.height = height;
    fmt.fmt.pix.pixelformat = V4L2_PIX_FMT_JPEG;
    fmt.fmt.pix.field = V4L2_FIELD_INTERLACED;
    if (ioctl (fd, VIDIOC_S_FMT, &fmt) == -1) {
        return -1;
    }
    return 0;
}

struct Buffer * ffi_create_buffers(int fd, int cnt) {
    struct Buffer * buffers = NULL;
    struct v4l2_requestbuffers req = {0};
    req.count = cnt;
    req.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    req.memory = V4L2_MEMORY_MMAP;

    if (ioctl(fd, VIDIOC_REQBUFS, &req) == -1) {
        free(buffers);
        return NULL;
    }
    buffers = (struct Buffer*) calloc (req.count, sizeof(struct Buffer));
    for (size_t idx = 0; idx < req.count; idx++) {
        struct v4l2_buffer buf = {0};
        buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        buf.memory = V4L2_MEMORY_MMAP;
        buf.index = idx;
        if(ioctl(fd, VIDIOC_QUERYBUF, &buf) == -1) {
            free(buffers);
            return NULL;
        }
        buffers[idx].len = buf.length;
        buffers[idx].start = mmap (NULL, buf.length, PROT_READ | PROT_WRITE, MAP_SHARED, fd, buf.m.offset);
        if (MAP_FAILED == buffers[idx].start) {
            free(buffers);
            return NULL;
        }
    }

    for (size_t idx = 0; idx < req.count; idx++) {
        struct v4l2_buffer buf = {};
 
        buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
        buf.memory = V4L2_MEMORY_MMAP;
        buf.index = idx;
        if (ioctl (fd, VIDIOC_QBUF, &buf) == -1) {
            free(buffers);
            return NULL;
        }
    }
    return buffers;
}

int ffi_start_streaming(int fd) {
    enum v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
 
    if (ioctl (fd, VIDIOC_STREAMON, &type) == -1) {
        return -1;
    }
    return 0;
}
int ffi_stop_streaming(int fd, struct Buffer * buffers, int cnt) {
    for (int idx = 0; idx < cnt; idx++) {
        munmap (buffers->start, buffers->len);
    }
       
    enum v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
 
    if (ioctl (fd, VIDIOC_STREAMOFF, &type) == -1) {
        return -1;
    }
    return 0;
}

int ffi_read_frame(int fd, struct Buffer * buffers, void (*callback)(const struct Buffer)) {
    struct v4l2_buffer buf;
    buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    buf.memory = V4L2_MEMORY_MMAP;

    int ff = ioctl (fd, VIDIOC_DQBUF, &buf);
    if (ff == EAGAIN){
        return 0;
    }
    if(ff == -1) {
        return -1;
    }

    callback(buffers[buf.index]);

    if (ioctl (fd, VIDIOC_QBUF, &buf) == -1){
        return -1;
    } 
    return 0;
}