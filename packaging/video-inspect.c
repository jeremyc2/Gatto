/* Gatto's minimal inspection entry point, compiled into the FFmpeg executable.
 * SPDX-License-Identifier: GPL-3.0-or-later
 * No general ffprobe UI, metadata tags, filenames, or arbitrary strings emitted.
 */
#include <math.h>
#include <stdio.h>
#include <string.h>
#include "libavformat/avformat.h"
#include "libavcodec/avcodec.h"
#include "libavutil/display.h"
#include "libavutil/pixdesc.h"

static int gatto_inspect(const char *path)
{
    AVFormatContext *input = NULL;
    AVDictionary *options = NULL;
    int result;
    av_log_set_level(AV_LOG_ERROR);
    /* Caller passes an absolute local filename. Also constrain nested inputs. */
    if (path[0] != '/') return 1;
    av_dict_set(&options, "protocol_whitelist", "file,pipe", 0);
    result = avformat_open_input(&input, path, NULL, &options);
    av_dict_free(&options);
    if (result < 0) goto finish;
    result = avformat_find_stream_info(input, NULL);
    if (result < 0 || input->nb_streams > 1024) { result = -1; goto finish; }
    printf("{\"format\":{\"duration\":\"%.9f\"},\"streams\":[",
           input->duration == AV_NOPTS_VALUE ? 0.0 : input->duration / (double)AV_TIME_BASE);
    for (unsigned i = 0; i < input->nb_streams; i++) {
        const AVStream *stream = input->streams[i];
        const AVCodecParameters *codec = stream->codecpar;
        const char *type = av_get_media_type_string(codec->codec_type);
        const char *transfer = av_color_transfer_name(codec->color_trc);
        int separator = 0;
        printf("%s{\"index\":%u,\"codec_type\":\"%s\",\"codec_name\":\"%s\","
               "\"width\":%d,\"height\":%d,\"color_transfer\":\"%s\",\"start_time\":",
               i ? "," : "", i, type ? type : "unknown", avcodec_get_name(codec->codec_id),
               codec->width, codec->height, transfer ? transfer : "unknown");
        if (stream->start_time == AV_NOPTS_VALUE) printf("null");
        else printf("\"%.9f\"", stream->start_time * av_q2d(stream->time_base));
        printf(",\"side_data_list\":[");
        for (int j = 0; j < codec->nb_coded_side_data; j++) {
            const AVPacketSideData *data = &codec->coded_side_data[j];
            const char *kind = NULL;
            if (data->type == AV_PKT_DATA_DISPLAYMATRIX && data->size >= 9 * sizeof(int32_t)) {
                double rotation = av_display_rotation_get((const int32_t *)data->data);
                if (isfinite(rotation)) {
                    printf("%s{\"rotation\":%.9f}", separator ? "," : "", rotation);
                    separator = 1;
                }
                continue;
            }
            if (data->type == AV_PKT_DATA_MASTERING_DISPLAY_METADATA) kind = "Mastering display metadata";
            if (data->type == AV_PKT_DATA_CONTENT_LIGHT_LEVEL) kind = "Content light level metadata";
            if (data->type == AV_PKT_DATA_DOVI_CONF) kind = "DOVI configuration record";
            if (kind) {
                printf("%s{\"side_data_type\":\"%s\"}", separator ? "," : "", kind);
                separator = 1;
            }
        }
        printf("]}");
    }
    printf("]}\n");
    result = ferror(stdout) ? -1 : 0;
finish:
    avformat_close_input(&input);
    return result < 0 ? 1 : 0;
}
