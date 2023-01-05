using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Net.Sockets;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace WLED_Controller
{
    public partial class Form1 : Form
    {
        public Form1()
        {
            InitializeComponent();
        }

        static UdpClient udpListener = new UdpClient();
        static int localPort = 65506;
        static IPEndPoint localEP = new IPEndPoint(IPAddress.Any, localPort);

        private void DataReceived(IAsyncResult ar)
        {
            UdpClient c = (UdpClient)ar.AsyncState;
            IPEndPoint receivedIpEndPoint = new IPEndPoint(IPAddress.Any, 0);
            Byte[] receivedBytes = c.EndReceive(ar, ref receivedIpEndPoint);
            c.BeginReceive(DataReceived, ar.AsyncState);

            // parse UDP data
            if (receivedBytes.Length > 0)
            {
                String ipAddress = receivedBytes[2] + "." + receivedBytes[3] + "." + receivedBytes[4] + "." + receivedBytes[5];

                    this.Invoke((MethodInvoker)delegate
                    {
                        // check if we already have this device in the list
                    var item = listView1.FindItemWithText(ipAddress);

                    // check if device was found
                    if (item == null)
                    {
                        // get info about WLED device
                        var wledInfo = GetInfo(ipAddress);
                        WledInstance instance = JsonConvert.DeserializeObject<WledInstance>(wledInfo);

                        TimeSpan t = TimeSpan.FromSeconds(instance.uptime);

                        // create single row
                        string[] row = { instance.name, instance.ip, instance.ver, instance.arch, t.ToString(@"dd\:hh\:mm\:ss")};

                        // create list view item
                        ListViewItem newListItem = new ListViewItem(row);

                        // add item to listview
                        listView1.Items.Add(newListItem);
                    }
                });
            }
        }

        private void Form1_Load(object sender, EventArgs e)
        {
            udpListener.ExclusiveAddressUse = false;
            udpListener.Client.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
            udpListener.Client.Bind(localEP);
            udpListener.BeginReceive(DataReceived, udpListener);
        }

        private string GetInfo(string ip)
        {
            string checkResult = null;
            var httpClient = new HttpClient();

            try
            {
                Task<string> t = httpClient.GetStringAsync("http://" + ip + "/json/info");
                checkResult = t.Result;
            }
            catch
            {
            }

            httpClient.Dispose();

            return checkResult;
        }

        private void button1_Click(object sender, EventArgs e)
        {
            // check if listview item is selected
            if(listView1.SelectedItems.Count > 0)
            {
                OpenBrowser("http://" + listView1.SelectedItems[0].SubItems[1].Text);
            }
        }

        private void OpenBrowser(string url)
        {
            try
            {
                System.Diagnostics.Process.Start(url);
            }
            catch (Exception ex)
            {
            }
        }

        private void listView1_SelectedIndexChanged(object sender, EventArgs e)
        {
            if (listView1.SelectedItems.Count > 0)
            {
                button1.Enabled = true;
            }
            else
            {
                button1.Enabled = false;
            }
        }

        private void button2_Click(object sender, EventArgs e)
        {
            listView1.Items.Clear();
        }
    }
}
