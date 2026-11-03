export type Faction = "shu" | "wei" | "wu" | "qun"

export interface Person {
  id: number
  name: string
  style: string
  faction: Faction
  factionLabel: string
  role: string
  birth: string
  bio: string
}

export const factionClass: Record<Faction, string> = {
  shu: "f-shu",
  wei: "f-wei",
  wu: "f-wu",
  qun: "f-qun",
}

export const people: Person[] = [
  { id: 1, name: "刘备", style: "玄德", faction: "shu", factionLabel: "蜀", role: "君主", birth: "161-223", bio: "蜀汉开国皇帝，汉中山靖王之后" },
  { id: 2, name: "关羽", style: "云长", faction: "shu", factionLabel: "蜀", role: "武将", birth: "?-220", bio: "五虎上将之首，温酒斩华雄，千里走单骑" },
  { id: 3, name: "张飞", style: "翼德", faction: "shu", factionLabel: "蜀", role: "武将", birth: "?-221", bio: "五虎上将之一，长坂桥喝退曹军" },
  { id: 4, name: "诸葛亮", style: "孔明", faction: "shu", factionLabel: "蜀", role: "谋士", birth: "181-234", bio: "卧龙先生，隆中对定三分天下" },
  { id: 5, name: "赵云", style: "子龙", faction: "shu", factionLabel: "蜀", role: "武将", birth: "?-229", bio: "五虎上将之一，长坂坡七进七出" },
  { id: 6, name: "曹操", style: "孟德", faction: "wei", factionLabel: "魏", role: "君主", birth: "155-220", bio: "魏武帝，挟天子以令诸侯" },
  { id: 7, name: "司马懿", style: "仲达", faction: "wei", factionLabel: "魏", role: "谋士", birth: "179-251", bio: "晋朝奠基人，与诸葛亮斗智" },
  { id: 8, name: "孙权", style: "仲谋", faction: "wu", factionLabel: "吴", role: "君主", birth: "182-252", bio: "吴大帝，赤壁之战联军破曹" },
  { id: 9, name: "周瑜", style: "公瑾", faction: "wu", factionLabel: "吴", role: "谋士", birth: "175-210", bio: "美周郎，赤壁之战指挥者" },
  { id: 10, name: "吕布", style: "奉先", faction: "qun", factionLabel: "群雄", role: "武将", birth: "?-199", bio: "人中吕布，马中赤兔" },
  { id: 11, name: "马超", style: "孟起", faction: "shu", factionLabel: "蜀", role: "武将", birth: "176-222", bio: "五虎上将之一，西凉锦马超" },
  { id: 12, name: "黄忠", style: "汉升", faction: "shu", factionLabel: "蜀", role: "武将", birth: "?-220", bio: "五虎上将之一，老当益壮" },
  { id: 13, name: "魏延", style: "文长", faction: "shu", factionLabel: "蜀", role: "武将", birth: "?-234", bio: "蜀汉大将，子午谷奇谋提出者" },
  { id: 14, name: "姜维", style: "伯约", faction: "shu", factionLabel: "蜀", role: "武将", birth: "202-264", bio: "天水麒麟儿，孔明衣钵传人" },
  { id: 15, name: "庞统", style: "士元", faction: "shu", factionLabel: "蜀", role: "谋士", birth: "179-214", bio: "凤雏先生，落凤坡遇伏身亡" },
  { id: 16, name: "黄月英", style: "—", faction: "shu", factionLabel: "蜀", role: "谋士", birth: "—", bio: "诸葛亮之妻，相传通晓机关术" },
  { id: 17, name: "法正", style: "孝直", faction: "shu", factionLabel: "蜀", role: "谋士", birth: "176-220", bio: "蜀汉谋士，定军山献计" },
  { id: 18, name: "马谡", style: "幼常", faction: "shu", factionLabel: "蜀", role: "谋士", birth: "190-228", bio: "失街亭，挥泪斩首" },
  { id: 19, name: "关平", style: "—", faction: "shu", factionLabel: "蜀", role: "武将", birth: "?-220", bio: "关羽义子，麦城同殉难" },
  { id: 20, name: "夏侯惇", style: "元让", faction: "wei", factionLabel: "魏", role: "武将", birth: "?-220", bio: "盲夏侯，拔矢啖睛" },
  { id: 21, name: "夏侯渊", style: "妙才", faction: "wei", factionLabel: "魏", role: "武将", birth: "?-219", bio: "虎步关右，定军山战死" },
  { id: 22, name: "张辽", style: "文远", faction: "wei", factionLabel: "魏", role: "武将", birth: "169-222", bio: "五子良将之首，逍遥津威震" },
  { id: 23, name: "许褚", style: "仲康", faction: "wei", factionLabel: "魏", role: "武将", birth: "?-?", bio: "虎痴，裸衣斗马超" },
  { id: 24, name: "典韦", style: "—", faction: "wei", factionLabel: "魏", role: "武将", birth: "?-197", bio: "古之恶来，宛城护主战死" },
  { id: 25, name: "徐晃", style: "公明", faction: "wei", factionLabel: "魏", role: "武将", birth: "?-227", bio: "五子良将之一，周亚夫之风" },
  { id: 26, name: "张郃", style: "儁乂", faction: "wei", factionLabel: "魏", role: "武将", birth: "?-231", bio: "五子良将之一，木门道中伏" },
  { id: 27, name: "郭嘉", style: "奉孝", faction: "wei", factionLabel: "魏", role: "谋士", birth: "170-207", bio: "鬼才，遗计定辽东" },
  { id: 28, name: "荀彧", style: "文若", faction: "wei", factionLabel: "魏", role: "谋士", birth: "163-212", bio: "王佐之才，曹操首席谋臣" },
  { id: 29, name: "黄盖", style: "公覆", faction: "wu", factionLabel: "吴", role: "武将", birth: "?-?", bio: "苦肉计诈降，赤壁火攻" },
  { id: 30, name: "陆逊", style: "伯言", faction: "wu", factionLabel: "吴", role: "谋士", birth: "183-245", bio: "夷陵之战火烧连营" },
]

export interface TabItem {
  id: string
  title: string
  kind: "table" | "query"
  sub?: string
  unsaved?: boolean
}

export type RowStyle = "plain" | "zebra" | "triple"

export const sqlFiles = [
  { name: "03_create_products.sql", tag: "oracle21c" },
  { name: "10_create_view.sql", tag: "oracle21c" },
  { name: "14_list_columns.sql", tag: "oracle21c" },
  { name: "query_10.sql", tag: "dbx-redis" },
  { name: "04_create_indexes.sql", tag: "oracle21c" },
  { name: "07_seed_orders.sql", tag: "oracle21c" },
  { name: "09_query_user_orders.sql", tag: "oracle21c" },
]

export const historyItems = [
  { sql: "SELECT * FROM public.sanguo_characters WHERE faction IN (...)", meta: "今天 14:32 · 38 行 · 6 ms", warn: false },
  { sql: "INSERT INTO public.sanguo_characters (name, style_name, ...)", meta: "今天 14:30 · 12 行 · 2 ms", warn: false },
  { sql: "CREATE TABLE public.sanguo_characters (...)", meta: "今天 14:29 · 完成", warn: false },
  { sql: "SELECT * FROM public.geo_test ORDER BY id DESC", meta: "昨天 18:04 · 0 行 · 2 ms", warn: true },
  { sql: "EXPLAIN ANALYZE SELECT * FROM spatial_ref_sys", meta: "昨天 17:48 · 8,500 行 · 41 ms", warn: false },
]
