--prints a csv from github repo transitland-atlas/feeds with data "feed,vehicles_url,trips_url,alerts_url,isauth,isswiftly"
local json = require("json")

function loadJsonFile(filePath)
    local file = io.open(filePath, "r")
    local contents = ""
    local myTable = {}
    if file ~= nil then
        local contents = file:read( "*a" )
        myTable = json.decode(contents);
        io.close(file)
        return myTable
    end
end

local directoryPath = "/home/lolpro11/Documents/transitland-atlas/feeds/"
os.execute("bash -c 'cd "..directoryPath.."; git pull &> /dev/null'")
print("onestop,gtfs_static_current")
local file_list = io.popen("ls -1 "..directoryPath.."/*.json")
local output = file_list:read("*all")
file_list:close()
local json_files = {}
for line in output:gmatch("[^\r\n]+") do
    table.insert(json_files, line)
end

for i=1, #json_files do
    local jsonData = loadJsonFile(json_files[i])
    for i=1, #jsonData.feeds do
        local output_str = ""
        local data_feed = jsonData.feeds[i]
        if data_feed.spec == "gtfs" and data_feed.urls ~= nil then
            output_str = tostring(data_feed.id)
            if data_feed.urls.static_current ~= nil then
                output_str = output_str..","..data_feed.urls.static_current
            end
        end
        if output_str ~= "" then
            print(output_str)
        end
        ::done::
    end
end
