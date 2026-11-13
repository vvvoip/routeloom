# China and LAN preset

The canonical bundled policy is `apps/client/assets/china-lan.rules`. It is available on all five clients and applied explicitly to the selected saved profile. Policy order is significant: private/link-local CIDRs, local suffixes, overseas exceptions, Chinese suffixes/services, then `final proxy:default`.

Domain entries are a curated derivative of [V2Fly/domain-list-community](https://github.com/v2fly/domain-list-community), fetched 2026-11-13 19:15 UTC (2026-11-14 in China). That download used the upstream master branch; an upstream commit SHA was not recorded. The shipped policy is frozen in each RouteLoom release and is not downloaded or refreshed independently. Local CIDRs and the final routing action are RouteLoom policy choices.

The upstream MIT notice is bundled as `apps/client/assets/DOMAIN-LIST-LICENSE.txt` and shown under the client's Licenses page. New releases may update the preset; applying one replaces custom rules only after confirmation. It is not an exhaustive geography database. TCP and UDP, suffix boundaries, common mainland services and overseas exceptions are checked by the shared policy tests.

中文：此预设来自 V2Fly/domain-list-community 的精选条目，使用 MIT 许可证，快照日期为北京时间 2026-11-14。原始下载未记录上游提交 SHA；随 RouteLoom 版本固定发布，不会单独联网更新。它不是完整的地理位置数据库。许可证可在客户端“许可证”中查看。
